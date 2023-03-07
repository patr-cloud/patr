import { DeploymentValue, RoutingValue, UrlType } from "./models";
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
            if (url.protocol === "http:") {
                url.protocol = "https:";
                return Response.redirect(url.toString(), 301);
            }

            if (url.hostname.endsWith(env.ONPATR_DOMAIN)) {
                // onpatr_domain should not be used in managed url
                return await fetchOnPatrRequest(request, env, url);
            } else {
                return await fetchManagedUrlRequest(request, env, url);
            }
        } catch (exception) {
            console.error(
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
    // todo: wildcard hostname is not matched
    // todo: path string should be updated to wildcard matching
    const routesStr = await env.ROUTING.get(url.hostname);
    if (!routesStr) {
        return new Response("404 Not Found - HOST", { status: 404 });
    }

    const routes = JSON.parse(routesStr) as RoutingValue;
    let matched_route: UrlType | null = null;
    for (const route of routes) {
        if (url.pathname.startsWith(route.path)) {
            matched_route = route;
            break;
        }
    }

    if (!matched_route) {
        return new Response("404 Not Found - URL", { status: 404 });
    }

    switch (matched_route.type) {
        case "proxyDeployment":
            return fetchDeployment(
                request,
                env,
                matched_route.deploymentId,
                matched_route.port
            );
        case "proxyStaticSite":
            return fetchStaticSite(request, env, matched_route.staticSiteId);
        case "proxyUrl":
            return proxyUrl(
                request,
                env,
                matched_route.path,
                matched_route.url,
                matched_route.httpOnly
            );
        case "redirect":
            return redirectUrl(
                request,
                env,
                matched_route.path,
                matched_route.url,
                matched_route.permanent_redirect,
                matched_route.httpOnly
            );
        default:
            return new Response(
                "500 Internal Server Error - Invalid router type",
                { status: 500 }
            );
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
            {
                status: 500,
            }
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

            // todo: need to add extra headers like client ip
            // see: https://fly.io/docs/reference/runtime-environment/#request-headers
            // see: https://developers.cloudflare.com/fundamentals/get-started/reference/http-request-headers/

            let headers = new Headers(request.headers);
            headers.set('Patr-Forwarded-For', new URL(request.url).hostname);

            let modified_request = new Request(destination_url, {
                method: request.method,
                headers,
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
                {
                    status: 500,
                }
            );
        }
    } else {
        return new Response(
            "500 Internal Server Error - Invalid deployment type",
            {
                status: 500,
            }
        );
    }
}

async function proxyUrl(
    request: Request,
    env: Env,
    matched_path: string,
    to_url: string,
    httpOnly: boolean
): Promise<Response> {
    let incomingUrl = new URL(request.url);

    let destinationUrl = httpOnly
        ? new URL(`http://${to_url}`)
        : new URL(`https://${to_url}`);
    destinationUrl.search = incomingUrl.search;
    destinationUrl.pathname =
        destinationUrl.pathname +
        incomingUrl.pathname.substring(matched_path.length);
    destinationUrl.hash = incomingUrl.hash;

    return fetch(
        new Request(destinationUrl, {
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
    matched_path: string,
    to_url: string,
    permanent_redirect: boolean,
    httpOnly: boolean
): Promise<Response> {
    // see: https://developers.cloudflare.com/workers/examples/redirect/

    let incomingUrl = new URL(request.url);

    let destinationUrl = httpOnly
        ? new URL(`http://${to_url}`)
        : new URL(`https://${to_url}`);
    destinationUrl.search = incomingUrl.search;
    destinationUrl.pathname =
        destinationUrl.pathname +
        incomingUrl.pathname.substring(matched_path.length);
    destinationUrl.hash = incomingUrl.hash;

    let redirectStatus = permanent_redirect ? 301 : 302;

    return Response.redirect(destinationUrl.toString(), redirectStatus);
}
