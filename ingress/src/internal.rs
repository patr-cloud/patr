use models::api::workspace::deployment::DeploymentStatus;

use crate::{
	prelude::*,
	utils::{build_forwarded_headers, serve_error_page},
};

/// The function that handles incoming requests to the ingress. It looks up the
/// deployment information from KV storage based on the host header, checks the
/// deployment status and routes the request to the appropriate runner if the
/// deployment is running. If any step fails, it serves an error page with the
/// appropriate status code.
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
		return serve_error_page("not-found", 404).await;
	};

	match kv {
		InternalKVData::Deployment {
			ports,
			runner_id,
			status,
		} => {
			if !matches!(status, DeploymentStatus::Running) {
				return serve_error_page("deployment-stopped", 503).await;
			}

			let Ok(port) = port.map(|port| port.parse::<u16>()).transpose() else {
				return Err(Error::RouteNoDataError);
			};

			let Some(port) = port.or(ports.iter().cloned().next()) else {
				console_debug!(
					"No port specified and multiple ports available for deployment ID {}",
					key
				);
				return serve_error_page("port-not-found", 404).await;
			};

			if !ports.contains(&port) {
				console_debug!("Port {} not found in deployment ports {:?}", port, ports);
				return serve_error_page("port-not-found", 404).await;
			}

			Fetch::Request(Request::new_with_init(
				req.url()?.as_str(),
				&RequestInit {
					body: req.inner().body().map(Into::into),
					headers: build_forwarded_headers(&req, "https")?,
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
			Fetch::Request(Request::new_with_init(
				req.url()?.as_str(),
				&RequestInit {
					body: req.inner().body().map(Into::into),
					headers: build_forwarded_headers(&req, "https")?,
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
