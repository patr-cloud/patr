import { Env } from ".";
import { StaticSiteValue } from "./models";

// reference: https://developers.cloudflare.com/pages/platform/serving-pages/

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

    const staticSiteStatusStr = await env.STATIC_SITE.get(staticSiteId);
    if (!staticSiteStatusStr) {
        return new Response(
            "500 Internal Server Error - Static Site not found",
            {
                status: 500,
            }
        );
    }

    const staticSiteStatus = JSON.parse(staticSiteStatusStr) as StaticSiteValue;
    if (staticSiteStatus === "created") {
        return new Response(
            "Static Site is created, try uploading some sites",
            {
                status: 404,
            }
        );
    } else if (staticSiteStatus === "deleted") {
        return new Response("Static Site is deleted", { status: 404 });
    } else if (staticSiteStatus === "stopped") {
        return new Response("Static Site is stopped", { status: 404 });
    } else if (staticSiteStatus.serving) {
        const filePrefix = `${staticSiteId}/${staticSiteStatus.serving}`;

        const requestedFilePath = decodeURIComponent(
            new URL(request.url).pathname
        )
            .split("/")
            .filter((s) => s !== "")
            .join("/");

        // todo:
        //  - range headers
        //  - cache
        //  - file matching logic

        const filePathToTry = requestedFilePath
            ? [
                  `${filePrefix}/${requestedFilePath}`,
                  `${filePrefix}/${requestedFilePath}/index.html`,
                  `${filePrefix}/${requestedFilePath}/index.htm`,
                  `${filePrefix}/index.html`,
                  `${filePrefix}/index.htm`,
                  `${filePrefix}/404.html`,
              ]
            : [
                  `${filePrefix}/index.html`,
                  `${filePrefix}/index.htm`,
                  `${filePrefix}/404.html`,
              ];

        for (const filePath of filePathToTry) {
            const fileMeta = await env.STATIC_SITE_STORAGE.head(filePath);
            if (!fileMeta) {
                continue;
            }

            const headers = [
                [
                    "content-type",
                    fileMeta.httpMetadata?.contentType ??
                        "application/octet-stream",
                ],
                ["content-length", fileMeta.size.toString()],
                ["etag", fileMeta.etag],
            ];

            const fileContent =
                request.method === "HEAD"
                    ? null
                    : await env.STATIC_SITE_STORAGE.get(filePath);

            return new Response(fileContent?.body, {
                status: 200,
                headers: headers,
            });
        }

        return new Response("404 Not Found - Requested file not found", {
            status: 404,
        });
    } else {
        return new Response(
            "500 Internal Server Error - Invalid static site type",
            { status: 500 }
        );
    }
}
