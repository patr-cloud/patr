async function handle(req) {
    const startTime = Date.now();
    const response = await req.subrequest(req.variables.origin, {
        args: req.args,
        body: req.body,
        method: req.method,
        detached: false,
    });
    const responseTime = (Date.now() - startTime) * 1.0;
    const url = new URL(req.uri);
    ngx.fetch("http://api.patr.cloud/webhook/deployment-request-log", {
        body: JSON.stringify({
            ipAddress: req.remoteAddr,
            method: req.method,
            domain: url.host,
            protocol: url.protcol,
            path: url.pathname,
            responseTime,
        }),
        method: "POST",
        headers: {
            "Content-Type": "application/json"
            // TODO custom header goes here
        }
    });
    req.rawHeadersOut = response.headers;
    req.return(response.status, await response.arrayBuffer());
}

export default handle;
