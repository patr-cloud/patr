use crate::prelude::*;

pub async fn handle_request(req: Request, env: Env, _ctx: Context, host: &str) -> Result<Response> {
	let host = host
		.trim_end_matches(constants::DEFAULT_PATR_DOMAIN)
		.trim_end_matches('.');

	if let Some((port, deployment_id)) = host.split_once('-') {
		let Ok(port) = port.parse::<u16>() else {
			return Err(Error::RouteNoDataError);
		};

		let Some(kv) = env
			.kv(constants::INGRESS_KV)?
			.get(deployment_id)
			.json::<DeploymentKVData>()
			.await?
		else {
			return Ok(Response::builder()
				.with_status(404)
				.body(ResponseBody::Body(b"Not found".to_vec())));
		};

		if !kv.ports.contains(&port) {
			return Ok(Response::builder()
				.with_status(404)
				.body(ResponseBody::Body(b"Not found".to_vec())));
		}

		Fetch::Request(Request::new_with_init(
			req.url()?.as_str(),
			&RequestInit {
				body: req.inner().body().map(Into::into),
				headers: req.headers().clone(),
				cf: CfProperties {
					resolve_override: Some(format!(
						"{}.{}",
						kv.runner_id,
						constants::DEFAULT_PATR_DOMAIN
					)),
					..Default::default()
				},
				method: req.method(),
				redirect: RequestRedirect::Manual,
			},
		)?)
		.send()
		.await
	} else {
		todo!()
	}
}
