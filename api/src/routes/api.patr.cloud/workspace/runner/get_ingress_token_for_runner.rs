use axum::http::StatusCode;
use cloudflare::{
	endpoints::cfd_tunnel::*,
	framework::{Environment, auth::Credentials, client::async_api::Client, response::ApiSuccess},
};
use models::api::workspace::runner::*;
use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// The configuration for the tunnel
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TunnelConfigRequest {
	/// This is the configuration that will be sent to Cloudflare
	config: TunnelConfigRequestConfig,
}

/// The list of ingress rules for the tunnel
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TunnelConfigRequestConfig {
	/// The list of ingress rules for the tunnel
	ingress: Vec<TunnelConfigRequestConfigIngress>,
}

/// The ingress rule for the tunnel
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TunnelConfigRequestConfigIngress {
	/// The service for the ingress rule. This is where the hostname will be
	/// pointed to
	service: String,
}

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
		user_data: _,
		state,
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

	let tunnel_id = runner.cloudflare_tunnel_id;

	let client = reqwest::Client::new();
	let cf_client = Client::new(
		Credentials::UserAuthToken {
			token: state.config.cloudflare.api_key.clone(),
		},
		Default::default(),
		Environment::Production,
	)?;

	let tunnel = client
		.get(format!(
			"https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel/{}",
			state.config.cloudflare.account_id, tunnel_id
		))
		.bearer_auth(&state.config.cloudflare.api_key)
		.send()
		.await?
		.json::<ApiSuccess<Option<Tunnel>>>()
		.await?
		.result
		.filter(|tunnel| tunnel.deleted_at.is_none());

	let tunnel = if let Some(tunnel) = tunnel {
		info!("Tunnel exists. Updating tunnel `{}`", tunnel.id);
		tunnel
	} else {
		// The tunnel does not exist. Create one
		info!("Tunnel does not exist. Creating tunnel");
		cf_client
			.request(&create_tunnel::CreateTunnel {
				account_identifier: &state.config.cloudflare.account_id,
				params: create_tunnel::Params {
					config_src: &ConfigurationSrc::Cloudflare,
					name: &format!("Runner: {}", runner_id),
					tunnel_secret: &b"default".to_vec(),
					metadata: None,
				},
			})
			.await?
			.result
	};

	query!(
		r#"
		UPDATE
			runner
		SET
			cloudflare_tunnel_id = $1
		WHERE
			id = $2;
		"#,
		tunnel.id.to_string(),
		runner_id as _,
	)
	.execute(&mut **database)
	.await?;

	debug!("Updating tunnel configuration to handle the runner");

	client
		.put(format!(
			"https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel/{}/configurations",
			state.config.cloudflare.account_id, tunnel.id
		))
		.bearer_auth(&state.config.cloudflare.api_key)
		.json(&TunnelConfigRequest {
			config: TunnelConfigRequestConfig {
				ingress: vec![TunnelConfigRequestConfigIngress {
					service: "unix:./data/nginx/nginx.sock".to_string(),
				}],
			},
		})
		.send()
		.await?
		.error_for_status()?;

	trace!("Getting the tunnel token for the runner");

	let token = client
		.get(format!(
			"https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel/{}/token",
			state.config.cloudflare.account_id, tunnel.id
		))
		.bearer_auth(&state.config.cloudflare.api_key)
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
