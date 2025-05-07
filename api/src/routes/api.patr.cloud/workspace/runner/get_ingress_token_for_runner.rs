use axum::http::StatusCode;
use cloudflare::{
	endpoints::{cfd_tunnel::*, dns::dns::*, zones::zone::*},
	framework::{
		Environment,
		SearchMatch,
		auth::Credentials,
		client::async_api::Client,
		response::ApiSuccess,
	},
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

	let tunnel_id = runner.cloudflare_tunnel_id;

	let client = reqwest::Client::new();
	let cf_client = Client::new(
		Credentials::UserAuthToken {
			token: config.cloudflare.api_key.clone(),
		},
		Default::default(),
		Environment::Production,
	)?;

	let tunnel = client
		.get(format!(
			"https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel/{}",
			config.cloudflare.account_id, tunnel_id
		))
		.bearer_auth(&config.cloudflare.api_key)
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
				account_identifier: &config.cloudflare.account_id,
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
			config.cloudflare.account_id, tunnel.id
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

	trace!("Getting the tunnel token for the runner");

	let token = client
		.get(format!(
			"https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel/{}/token",
			config.cloudflare.account_id, tunnel.id
		))
		.bearer_auth(&config.cloudflare.api_key)
		.send()
		.await?
		.json::<ApiSuccess<String>>()
		.await?
		.result;

	trace!("Updating DNS record for the tunnel");

	let zone_id = cf_client
		.request(&ListZones {
			params: ListZonesParams {
				name: Some(config.primary_hosted_domain.clone()),
				status: Some(Status::Active),
				search_match: Some(SearchMatch::All),
				..Default::default()
			},
		})
		.await?
		.result
		.into_iter()
		.next()
		.ok_or(ErrorType::ResourceDoesNotExist)
		.inspect_err(|_| {
			error!(
				"No zone exists for the domain `{}`",
				config.primary_hosted_domain
			);
		})?
		.id;

	if let Some(record) = cf_client
		.request(&ListDnsRecords {
			zone_identifier: &zone_id,
			params: ListDnsRecordsParams {
				name: Some(format!("{}.{}", runner_id, config.primary_hosted_domain)),
				..Default::default()
			},
		})
		.await?
		.result
		.into_iter()
		.next()
	{
		info!("DNS record for the runner exists. Updating it");
		cf_client
			.request(&UpdateDnsRecord {
				zone_identifier: &zone_id,
				identifier: &record.id,
				params: UpdateDnsRecordParams {
					name: &format!("{}.{}", runner_id, config.primary_hosted_domain),
					ttl: Some(0),
					proxied: Some(true),
					content: DnsContent::CNAME {
						content: format!("{}.cfargotunnel.com", tunnel.id),
					},
				},
			})
			.await?;
	} else {
		info!("DNS record for the runner does not exist. Creating a new one");
		cf_client
			.request(&CreateDnsRecord {
				zone_identifier: &zone_id,
				params: CreateDnsRecordParams {
					name: &format!("{}.{}", runner_id, config.primary_hosted_domain),
					ttl: Some(0),
					proxied: Some(true),
					priority: None,
					content: DnsContent::CNAME {
						content: format!("{}.cfargotunnel.com", tunnel.id),
					},
				},
			})
			.await?;
	}

	AppResponse::builder()
		.body(GetIngressTokenForRunnerResponse { token })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
