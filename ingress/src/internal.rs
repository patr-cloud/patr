use crate::prelude::*;

pub async fn handle_request(req: Request, env: Env, _ctx: Context, host: &str) -> Result<Response> {
	let host = host
		.trim_end_matches(constants::DEFAULT_PATR_DOMAIN)
		.trim_end_matches('.');

	let (port, key) = if let Some((port, deployment_id)) = host.rsplit_once('-') {
		(Some(port), deployment_id)
	} else {
		(None, host)
	};

	let Some(kv) = env
		.kv(constants::INGRESS_KV)?
		.get(key)
		.json::<InternalKVData>()
		.await?
	else {
		console_debug!("No KV data found for deployment ID {}", key);
		return Ok(Response::builder()
			.with_status(404)
			.body(ResponseBody::Body(b"Not found".to_vec())));
	};

	match kv {
		InternalKVData::Deployment {
			ports,
			runner_id,
			status: _,
		} => {
			let Ok(port) = port.map(|port| port.parse::<u16>()).transpose() else {
				return Err(Error::RouteNoDataError);
			};

			let Some(port) = port.or(ports.iter().cloned().next()) else {
				console_debug!(
					"No port specified and multiple ports available for deployment ID {}",
					key
				);
				return Ok(Response::builder()
					.with_status(404)
					.body(ResponseBody::Body(b"Not found".to_vec())));
			};

			if !ports.contains(&port) {
				console_debug!("Port {} not found in deployment ports {:?}", port, ports);
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
							runner_id,
							constants::DEFAULT_PATR_DOMAIN
						)),
						..Default::default()
					},
					method: req.method(),
					redirect: RequestRedirect::Manual,
					cache: None,
				},
			)?)
			.send()
			.await
		}
		InternalKVData::Runner => {
			let InternalKVData::Runner = kv else {
				console_error!("Expected a Runner KV data, but found {:?}", kv);
				return Ok(Response::builder()
					.with_status(404)
					.body(ResponseBody::Body(b"Internal server error".to_vec())));
			};

			Fetch::Request(Request::new_with_init(
				req.url()?.as_str(),
				&RequestInit {
					body: req.inner().body().map(Into::into),
					headers: req.headers().clone(),
					cf: Default::default(),
					method: req.method(),
					redirect: RequestRedirect::Manual,
					cache: None,
				},
			)?)
			.send()
			.await
		}
	}
}
