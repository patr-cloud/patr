use std::collections::BTreeMap;

use crate::{prelude::*, utils::serve_error_page};

/// Handles all requests to custom domains (managed URLs).
pub async fn handle_request(req: Request, env: Env, ctx: Context, host: &str) -> Result<Response> {
	let url = req.url()?;

	let Some(kv_value) = env
		.kv(constants::INGRESS_KV)?
		.get(host)
		.json::<BTreeMap<String, ManagedUrlKVData>>()
		.await?
	else {
		return serve_error_page("not-found", 404).await;
	};

	// Patr owns /.well-known/patr/* — used to verify managed URL is served by Patr
	if url.path() == "/.well-known/patr/managed-url" {
		return Response::ok("ok");
	}

	let Some((_mount_point, value)) = kv_value
		.into_iter()
		.filter(|(mount_point, value)| {
			if value.is_redirect() {
				url.path() == mount_point
			} else {
				url.path().starts_with(mount_point)
			}
		})
		.reduce(|(mount_point_a, value_a), (mount_point_b, value_b)| {
			if value_a.is_redirect() {
				return (mount_point_a, value_a);
			}
			if value_b.is_redirect() {
				return (mount_point_b, value_b);
			}
			if mount_point_a.len() > mount_point_b.len() {
				(mount_point_a, value_a)
			} else {
				(mount_point_b, value_b)
			}
		})
	else {
		return serve_error_page("not-found", 404).await;
	};

	match value {
		ManagedUrlKVData::Redirect {
			url,
			permanent_redirect,
			http_only,
		} => {
			let mut url = Url::parse(&url)?;

			url.set_scheme(if http_only { "http" } else { "https" })
				.map_err(|_| Error::BadEncoding)?;

			Response::redirect_with_status(
				url,
				if permanent_redirect {
					constants::STATUS_CODE_PERMANENT_REDIRECT
				} else {
					constants::STATUS_CODE_TEMPORAL_REDIRECT
				},
			)
		}

		ManagedUrlKVData::ProxyDeployment {
			deployment_id: _,
			port: _,
			runner_id,
		} => {
			Fetch::Request(Request::new_with_init(
				url.as_str(),
				&RequestInit {
					body: req.inner().body().map(Into::into),
					headers: req.headers().clone(),
					cf: CfProperties {
						minify: Some(MinifyConfig {
							js: false,
							html: false,
							css: false,
						}),
						polish: Some(PolishConfig::Off),
						resolve_override: Some(format!(
							"{}.{}",
							runner_id,
							constants::DEFAULT_PATR_DOMAIN
						)),
						scrape_shield: Some(true),
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
	}
}
