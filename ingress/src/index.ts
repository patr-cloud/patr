import { AwsClient } from "aws4fetch";
import { DeploymentValue, RoutingValue } from "./models";
import { fetchStaticSite } from "./staticSite";

export interface Env {
    // KVs
    ROUTING: KVNamespace;
    DEPLOYMENT: KVNamespace;
    STATIC_SITE: KVNamespace;

    // R2
    STATIC_SITE_STORAGE: R2Bucket;

    // ENVs
    ONPATR_DOMAIN: string;
}

export default {
    async fetch(
        request: Request,
        env: Env,
        ctx: ExecutionContext
    ): Promise<Response> {
        try {
            let url = new URL(request.url);
            if (url.hostname.endsWith(env.ONPATR_DOMAIN)) {
                // onpatr_domain should not be used in managed url
                return fetchOnPatrRequest(request, env, url);
            } else {
                return fetchManagedUrlRequest(request, env, url);
            }
        } catch (exception) {
            console.log(
                `Error while fetching request in cf ingress - ${exception}`
            );
            return new Response("500 Internal Server Error", { status: 500 });
        }
    },
};

async function fetchOnPatrRequest(
    request: Request,
    env: Env,
    url: URL
): Promise<Response> {
    let subdomain = url.host.substring(
        0,
        url.host.length - env.ONPATR_DOMAIN.length - 1
    );
    let [part1, part2] = subdomain.split("-", 2);

    if (part2) {
        return fetchDeployment(request, env, part2, parseInt(part1));
    } else {
        return fetchStaticSite(request, env, part1);
    }
}

async function fetchManagedUrlRequest(
    request: Request,
    env: Env,
    url: URL
): Promise<Response> {
    let routes = await env.ROUTING.get<RoutingValue>(url.hostname);
    if (routes) {
        // todo: need to use use customer route matching
        let matched_route = routes.get(url.pathname);
        if (matched_route) {
            switch (matched_route.type) {
                case "proxyDeployment":
                    return fetchDeployment(
                        request,
                        env,
                        matched_route.deploymentId,
                        matched_route.port
                    );
                case "proxyStaticSite":
                    return fetchStaticSite(
                        request,
                        env,
                        matched_route.staticSiteId
                    );
                case "proxyUrl":
                    return proxyUrl(request, env, "/", matched_route.url); // todo
                case "redirect":
                    return redirectUrl(
                        request,
                        env,
                        matched_route.url,
                        matched_route.permanent
                    );
                default:
                    return new Response(
                        "500 Internal Server Error - Invalid router type",
                        { status: 500 }
                    );
            }
        } else {
            return new Response("404 Not Found - URL", { status: 404 });
        }
    } else {
        return new Response("404 Not Found - HOST", { status: 404 });
    }
}

async function fetchDeployment(
    request: Request,
    env: Env,
    deploymentId: string,
    port: number
): Promise<Response> {
    let deploymentStatusStr = await env.DEPLOYMENT.get(deploymentId);

    if (!deploymentStatusStr) {
        return new Response(
            "500 Internal Server Error - Deployment not found",
            { status: 500 }
        );
    }

    let deploymentStatus = JSON.parse(deploymentStatusStr) as DeploymentValue;
    if (deploymentStatus === "created") {
        return new Response("Deployment is created, try starting it", {
            status: 404,
        });
    } else if (deploymentStatus === "deleted") {
        return new Response("Deployment is deleted", { status: 404 });
    } else if (deploymentStatus === "stopped") {
        return new Response("Deployment is stopped", { status: 404 });
    } else if (
        deploymentStatus.running &&
        deploymentStatus.running.ports &&
        deploymentStatus.running.regionId
    ) {
        if (deploymentStatus.running.ports.includes(port)) {
            let regionId = deploymentStatus.running.regionId;
            let destination_url = new URL(request.url);
            destination_url.host = `${port}-${deploymentId}.${deploymentStatus.running.regionId}.${env.ONPATR_DOMAIN}`;

            let modified_request = new Request(destination_url, {
                method: request.method,
                headers: request.headers,
                body: request.body,
                cf: request.cf,
                fetcher: request.fetcher,
                redirect: request.redirect,
                signal: request.signal,
            });

            return fetch(modified_request);
        } else {
            return new Response(
                "500 Internal Server Error - Port not running",
                { status: 500 }
            );
        }
    } else {
        return new Response(
            "500 Internal Server Error - Invalid deployment type",
            { status: 500 }
        );
    }
}

async function proxyUrl(
    request: Request,
    env: Env,
    matched_path: string,
    to_url: string
): Promise<Response> {
    // todo: make sure that it is okay to allow methods other than GET
    // todo: handle headers and other stuff
    // todo: use matched path and strip it accordingly

    let destination_url = new URL(to_url);
    return fetch(
        new Request(destination_url, {
            method: request.method,
            headers: request.headers,
            body: request.body,
            cf: request.cf,
            fetcher: request.fetcher,
            redirect: request.redirect,
            signal: request.signal,
        })
    );
}

async function redirectUrl(
    request: Request,
    env: Env,
    to_url: string,
    permanent: boolean
): Promise<Response> {
    // todo: make sure to_url is a domain or strip the common things
    // see: https://developers.cloudflare.com/workers/examples/redirect/

    let redirectStatus = permanent ? 301 : 302;
    return Response.redirect(to_url, redirectStatus);
}
