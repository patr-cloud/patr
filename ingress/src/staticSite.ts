import { Env } from ".";
import { StaticSiteValue } from "./models";

export async function fetchStaticSite(
    request: Request,
    env: Env,
    staticSiteId: string
): Promise<Response> {
    // allow only read methods
    switch (request.method) {
        case "GET":
        case "HEAD":
        case "OPTIONS":
            break;
        default:
            return new Response("405 Method Not Allowed", { status: 405 });
    }

    // handle options method
    if (request.method === "OPTIONS") {
        return new Response(null, { headers: { allow: "GET, HEAD, OPTIONS" } });
    }

    let staticSiteStatusStr = await env.STATIC_SITE.get(staticSiteId);

    if (!staticSiteStatusStr) {
        return new Response(
            "500 Internal Server Error - Static Site not found",
            { status: 500 }
        );
    }

    let staticSiteStatus = JSON.parse(staticSiteStatusStr) as StaticSiteValue;
    if (staticSiteStatus === "created") {
        return new Response(
            "Static Site is created, try uploading some sites",
            { status: 404 }
        );
    } else if (staticSiteStatus === "deleted") {
        return new Response("Static Site is deleted", { status: 404 });
    } else if (staticSiteStatus === "stopped") {
        return new Response("Static Site is stopped", { status: 404 });
    } else if (staticSiteStatus.serving) {
        let filePrefix = `${staticSiteId}/${staticSiteStatus.serving}`;

        let requestedFilePath = decodeURIComponent(
            new URL(request.url).pathname
        );

        let originalFilePath = `${filePrefix}${requestedFilePath}`;
        if (originalFilePath.endsWith("/")) {
            originalFilePath = originalFilePath + "index.html";
        }

        // todo: handle head method and range headers

        console.log("f1: ", originalFilePath);
        var file = await env.STATIC_SITE_STORAGE.get(originalFilePath);
        if (!file) {
            let indexFilePath = `${filePrefix}/index.html`;
            console.log("f2: ", indexFilePath);
            file = await env.STATIC_SITE_STORAGE.get(indexFilePath);
        }

        if (file?.body) {
            return new Response(file?.body, { status: 200, headers:  [["content-type", file.httpMetadata?.contentType ?? "application/octet-stream"]]});
        } else {
            return new Response("500 Not Found - Requested file not found", {
                status: 404,
            });
        }
    } else {
        return new Response(
            "500 Internal Server Error - Invalid static site type",
            { status: 500 }
        );
    }
}
