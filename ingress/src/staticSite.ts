import { Env } from ".";
import { StaticSiteValue } from "./models";

// reference: https://developers.cloudflare.com/pages/platform/serving-pages/

export async function fetchStaticSite(
    request: Request,
    env: Env,
    matched_path: string,
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
        return new Response("500 Internal Server Error - Static Site not found", {
            status: 500,
        });
    }

    const staticSiteStatus = JSON.parse(staticSiteStatusStr) as StaticSiteValue;
    if (staticSiteStatus === "created") {
        return new Response("Static Site is created, try uploading some sites", {
            status: 404,
        });
    } else if (staticSiteStatus === "deleted") {
        return new Response("Static Site is deleted", { status: 404 });
    } else if (staticSiteStatus === "stopped") {
        return new Response("Static Site is stopped", { status: 404 });
    } else if (staticSiteStatus.serving) {
        const filePrefix = `${staticSiteId}/${staticSiteStatus.serving}`;

        let destination_url = new URL(request.url);
        destination_url.pathname = destination_url.pathname.substring(
            matched_path.length
        );
        destination_url.pathname =
            destination_url.pathname[0] === "/"
                ? destination_url.pathname
                : "/" + destination_url.pathname;

        const requestedFilePath = decodeURIComponent(destination_url.pathname)
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
            const file = await env.STATIC_SITE_STORAGE.get(filePath);
            if (!file) {
                continue;
            }

            let extension = filePath.split(".").slice(-1);
            let mime_string = get_mime_type_from_file_name(extension[0] ?? "");

            const headers = [
                ["content-type", mime_string],
                ["content-length", file.size.toString()],
                ["etag", file.etag],
            ];

            return new Response(file?.body, {
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

function get_mime_type_from_file_name(file_extension: string): string {
    switch (file_extension) {
        case "html":
            return "text/html";
        case "htm":
            return "text/html";
        case "shtml":
            return "text/html";
        case "xhtml":
            return "application/xhtml+xml";
        case "css":
            return "text/css";
        case "xml":
            return "text/xml";
        case "atom":
            return "application/atom+xml";
        case "rss":
            return "application/rss+xml";
        case "js":
            return "application/javascript";
        case "mml":
            return "text/mathml";
        case "png":
            return "image/png";
        case "jpg":
            return "image/jpeg";
        case "jpeg":
            return "image/jpeg";
        case "gif":
            return "image/gif";
        case "ico":
            return "image/x-icon";
        case "svg":
            return "image/svg+xml";
        case "svgz":
            return "image/svg+xml";
        case "tif":
            return "image/tiff";
        case "tiff":
            return "image/tiff";
        case "json":
            return "application/json";
        case "pdf":
            return "application/pdf";
        case "txt":
            return "text/plain";
        case "mp4":
            return "video/mp4";
        case "webm":
            return "video/webm";
        case "mp3":
            return "audio/mpeg";
        case "ogg":
            return "audio/ogg";
        case "wav":
            return "audio/wav";
        case "woff":
            return "application/font-woff";
        case "woff2":
            return "application/font-woff2";
        case "ttf":
            return "application/font-truetype";
        case "otf":
            return "application/font-opentype";
        case "eot":
            return "application/vnd.ms-fontobject";
        case "mpg":
            return "video/mpeg";
        case "mpeg":
            return "video/mpeg";
        case "mov":
            return "video/quicktime";
        case "avi":
            return "video/x-msvideo";
        case "flv":
            return "video/x-flv";
        case "m4v":
            return "video/x-m4v";
        case "jad":
            return "text/vnd.sun.j2me.app-descriptor";
        case "wml":
            return "text/vnd.wap.wml";
        case "htc":
            return "text/x-component";
        case "avif":
            return "image/avif";
        case "webp":
            return "image/webp";
        case "wbmp":
            return "image/vnd.wap.wbmp";
        case "jng":
            return "image/x-jng";
        case "bmp":
            return "image/x-ms-bmp";
        case "jar":
            return "application/java-archive";
        case "war":
            return "application/java-archive";
        case "ear":
            return "application/java-archive";
        case "hqx":
            return "application/mac-binhex40";
        case "doc":
            return "application/msword";
        case "ps":
            return "application/postscript";
        case "eps":
            return "application/postscript";
        case "ai":
            return "application/postscript";
        case "rtf":
            return "application/rtf";
        case "m3u8":
            return "application/vnd.apple.mpegurl";
        case "kml":
            return "application/vnd.google-earth.kml+xml";
        case "kmz":
            return "application/vnd.google-earth.kmz";
        case "xls":
            return "application/vnd.ms-excel";
        case "ppt":
            return "application/vnd.ms-powerpoint";
        case "odg":
            return "application/vnd.oasis.opendocument.graphics";
        case "odp":
            return "application/vnd.oasis.opendocument.presentation";
        case "ods":
            return "application/vnd.oasis.opendocument.spreadsheet";
        case "odt":
            return "application/vnd.oasis.opendocument.text";
        case "pptx":
            return "application/vnd.openxmlformats-officedocument.presentationml.presentation";
        case "xlsx":
            return "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
        case "docx":
            return "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
        case "wmlc":
            return "application/vnd.wap.wmlc";
        case "wasm":
            return "application/wasm";
        case "7z":
            return "application/x-7z-compressed";
        case "cco":
            return "application/x-cocoa";
        case "jardiff":
            return "application/x-java-archive-diff";
        case "jnlp":
            return "application/x-java-jnlp-file";
        case "run":
            return "application/x-makeself";
        case "pl":
            return "application/x-perl";
        case "pm":
            return "application/x-perl";
        case "prc":
            return "application/x-pilot";
        case "pdb":
            return "application/x-pilot";
        case "rar":
            return "application/x-rar-compressed";
        case "rpm":
            return "application/x-redhat-package-manager";
        case "sea":
            return "application/x-sea";
        case "swf":
            return "application/x-shockwave-flash";
        case "sit":
            return "application/x-stuffit";
        case "tcl":
            return "application/x-tcl";
        case "tk":
            return "application/x-tcl";
        case "der":
            return "application/x-x509-ca-cert";
        case "pem":
            return "application/x-x509-ca-cert";
        case "crt":
            return "application/x-x509-ca-cert";
        case "xpi":
            return "application/x-xpinstall";
        case "xspf":
            return "application/xspf+xml";
        case "zip":
            return "application/zip";
        case "bin":
            return "application/octet-stream";
        case "exe":
            return "application/octet-stream";
        case "dll":
            return "application/octet-stream";
        case "deb":
            return "application/octet-stream";
        case "dmg":
            return "application/octet-stream";
        case "iso":
            return "application/octet-stream";
        case "img":
            return "application/octet-stream";
        case "msi":
            return "application/octet-stream";
        case "msp":
            return "application/octet-stream";
        case "msm":
            return "application/octet-stream";
        case "mid":
            return "audio/midi";
        case "midi":
            return "audio/midi";
        case "kar":
            return "audio/midi";
        case "m4a":
            return "audio/x-m4a";
        case "ra":
            return "audio/x-realaudio";
        case "3gpp":
            return "video/3gpp";
        case "3gp":
            return "video/3gpp";
        case "ts":
            return "video/mp2t";
        case "mng":
            return "video/x-mng";
        case "asx":
            return "video/x-ms-asf";
        case "asf":
            return "video/x-ms-asf";
        case "wmv":
            return "video/x-ms-wmv";
        default:
            return "application/octet-stream";
    }
}
