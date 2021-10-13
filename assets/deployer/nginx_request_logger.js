function handle(req) {
    const startTime = Date.now();
    req.subrequest(req.variables.origin, {
        args: req.args,
        body: req.body,
        method: req.method,
        detached: false,
    }).then(resp => {
        const responseTime = (Date.now() - startTime) * 1.0;
        
        ngx.fetch("http://api.patr.cloud/webhook/deployment-request-log", {
            body: JSON.stringify({
                ipAddress: req.remoteAddress,
                method: req.method,
                domain: req.headersIn.host,
                protocol: "https",
                path: req.uri,
                responseTime,
            }),
            method: req.method,
            headers: {
                "Content-Type": "application/json"
                // TODO custom header goes here
            }
        });
        req.headersOut = resp.headers;
        resp.arrayBuffer().then(buffer => {
            req.return(resp.status, buffer);
        });
    }).catch(err => {
        req.error(`caught error '${err}' during sending request with auth service'`)
    });
}

export default { handle };

