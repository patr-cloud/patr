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
	/// The hostname for the ingress rule
	#[serde(skip_serializing_if = "String::is_empty")]
	hostname: String,
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
				body: GetIngressTokenForRunnerRequestProcessed { runner_port },
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
		.bearer_auth(&config.cloudflare.api_key)
		.send()
		.await?
		.json::<ApiSuccess<Option<Tunnel>>>()
		.await?
		.result
		.filter(|tunnel| tunnel.deleted_at.is_none());

	let tunnel = if let Some(tunnel) = tunnel {
		tunnel
	} else {
		// The tunnel does not exist. Create one
		Client::new(
			Credentials::UserAuthToken {
				token: config.cloudflare.api_key.clone(),
			},
			Default::default(),
			Environment::Production,
		)?
		.request(&create_tunnel::CreateTunnel {
			account_identifier: &account_id,
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

	client
		.put(format!(
			"https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel/{}/configurations",
			account_id, tunnel.id
		))
		.bearer_auth(&config.cloudflare.api_key)
		.json(&TunnelConfigRequest {
			config: TunnelConfigRequestConfig {
				ingress: vec![
					TunnelConfigRequestConfigIngress {
						hostname: format!("{}.{}", runner_id, config.primary_hosted_domain),
						service: format!("http://localhost:{}", runner_port),
					},
					TunnelConfigRequestConfigIngress {
						hostname: String::new(),
						service: "http_status:404".to_string(),
					},
				],
			},
		})
		.send()
		.await?
		.error_for_status()?;

	let token = client
		.get(format!(
			"https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel/{}/token",
			account_id, tunnel.id
		))
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
