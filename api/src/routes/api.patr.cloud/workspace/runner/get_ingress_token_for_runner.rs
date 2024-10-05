use axum::http::{HeaderName, HeaderValue, StatusCode};
use cloudflare::{
	endpoints::cfd_tunnel::*,
	framework::{async_api::Client, auth::Credentials, response::ApiSuccess, Environment},
};
use models::api::workspace::runner::*;

use crate::prelude::*;

pub async fn get_ingress_token_for_runner(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetIngressTokenForRunnerPath {
					workspace_id,
					runner_id,
				},
				query: (),
				headers:
					GetIngressTokenForRunnerRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetIngressTokenForRunnerRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data: _,
	}: AuthenticatedAppRequest<'_, GetIngressTokenForRunnerRequest>,
) -> Result<AppResponse<GetIngressTokenForRunnerRequest>, ErrorType> {
	info!("Getting ingress token for runner `{runner_id}`");

	let runner = query!(
		r#"
		SELECT
			*
		FROM
			runner
		WHERE
			id = $1 AND
            workspace_id = $2 AND
			deleted IS NULL;
		"#,
		&runner_id as _,
		&workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let account_id = config.cloudflare.account_id;
	let tunnel_id = runner.cloudflare_tunnel_id;

	let client = reqwest::Client::new();

	let tunnel = client
		.get(format!(
			"https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel/{}",
			account_id, tunnel_id
		))
		.header(
			HeaderName::from_static("x-auth-email"),
			HeaderValue::from_str(&config.cloudflare.email)?,
		)
		.bearer_auth(&config.cloudflare.api_key)
		.send()
		.await?
		.json::<ApiSuccess<Option<Tunnel>>>()
		.await?
		.result;

	// If None, return true.
	// If Some, return true if deleted_at is Some
	if tunnel.map_or(true, |tunnel| tunnel.deleted_at.is_some()) {
		// The tunnel does not exist. Create one
		Client::new(
			Credentials::UserAuthKey {
				email: config.cloudflare.email.clone(),
				key: config.cloudflare.api_key.clone(),
			},
			Default::default(),
			Environment::Production,
		)?
		.request(&create_tunnel::CreateTunnel {
			account_identifier: &account_id,
			params: create_tunnel::Params {
				config_src: &ConfigurationSrc::Cloudflare,
				name: &format!("Runner: {}", runner_id),
				tunnel_secret: &Default::default(),
				metadata: None,
			},
		})
		.await?
		.result;
	}

	let token = client
		.get(format!(
			"https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel/{}/token",
			account_id, tunnel_id
		))
		.header(
			HeaderName::from_static("x-auth-email"),
			HeaderValue::from_str(&config.cloudflare.email)?,
		)
		.bearer_auth(&config.cloudflare.api_key)
		.send()
		.await?
		.json::<ApiSuccess<String>>()
		.await?
		.result;

	AppResponse::builder()
		.body(GetIngressTokenForRunnerResponse { token })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
