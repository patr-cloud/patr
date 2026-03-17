use axum::Router;
use cloudflare::{
	endpoints::cfd_tunnel::*,
	framework::{Environment, auth::Credentials, client::async_api::Client, response::ApiSuccess},
};
use serde::{Deserialize, Serialize};

use crate::{prelude::*, utils::config::AppConfig};

mod add_runner_to_workspace;
mod get_ingress_token_for_runner;
mod get_runner_info;
mod list_runners_for_workspace;
mod remove_runner_from_workspace;
mod stream_runner_data_for_workspace;

use self::{
	add_runner_to_workspace::*,
	get_ingress_token_for_runner::*,
	get_runner_info::*,
	list_runners_for_workspace::*,
	remove_runner_from_workspace::*,
	stream_runner_data_for_workspace::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.mount_auth_endpoint(add_runner_to_workspace, state, allowed_client_type)
		.mount_auth_endpoint(get_ingress_token_for_runner, state, allowed_client_type)
		.mount_auth_endpoint(get_runner_info, state, allowed_client_type)
		.mount_auth_endpoint(list_runners_for_workspace, state, allowed_client_type)
		.mount_auth_endpoint(remove_runner_from_workspace, state, allowed_client_type)
		.mount_auth_endpoint(stream_runner_data_for_workspace, state, allowed_client_type)
}

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
	/// The hostname for the ingress rule. This is the hostname that will be
	/// pointed to the runner. If this is `None`, this ingress rule will be a
	/// catch-all rule that will match all hostnames
	hostname: Option<String>,
	/// The service for the ingress rule. This is where the hostname will be
	/// pointed to
	service: String,
}

/// Sync the Cloudflare configuration for a runner. This will query the database
/// for all deployments running on the runner, and update the Cloudflare
/// configuration accordingly.
pub async fn update_cloudflare_config_for_runner(
	runner_id: Uuid,
	database: &mut DatabaseConnection,
	config: &AppConfig,
) -> Result<(), ErrorType> {
	let runner = query!(
		r#"
		SELECT
			*
		FROM
			runner
		WHERE
			id = $1;
		"#,
		&runner_id as _,
	)
	.fetch_optional(&mut *database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let tunnel_id = runner.cloudflare_tunnel_id;

	let client = reqwest::Client::new();
	let cf_client = Client::new(
		Credentials::UserAuthToken {
			token: config.cloudflare.api_key.clone(),
		},
		Default::default(),
		Environment::Custom(config.cloudflare.base_url.clone()),
	)?;

	let tunnel = client
		.get(format!(
			"{}accounts/{}/cfd_tunnel/{}",
			config.cloudflare.base_url, config.cloudflare.account_id, tunnel_id
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
	.execute(&mut *database)
	.await?;

	// Get all deployments and their ports that are running on the runner
	let deployment_ports = query!(
		r#"
		SELECT
			deployment_exposed_port.deployment_id AS "deployment_id: Uuid",
			deployment_exposed_port.port
		FROM
			deployment_exposed_port
		INNER JOIN
			deployment
		ON
			deployment_exposed_port.deployment_id = deployment.id
		WHERE
			deployment.runner = $1;
		"#,
		runner_id as _,
	)
	.fetch_all(&mut *database)
	.await?
	.into_iter()
	.map(|record| {
		(
			record.deployment_id,
			record.port,
			format!("{}-{}.onpatr.cloud", record.port, record.deployment_id),
		)
	});

	let managed_url_ports = query!(
		r#"
		SELECT
			managed_url.deployment_id AS "deployment_id!: Uuid",
			managed_url.port AS "port!",
			CONCAT(
				managed_url.sub_domain,
				'.',
				workspace_domain.name,
				'.',
				workspace_domain.tld
			) AS "host!"
		FROM
			managed_url
		INNER JOIN
			deployment
		ON
			managed_url.deployment_id = deployment.id
		INNER JOIN
			workspace_domain
		ON
			managed_url.domain_id = workspace_domain.id
		WHERE
			managed_url.deployment_id IS NOT NULL AND
			managed_url.port IS NOT NULL AND
			deployment.runner = $1;
		"#,
		runner_id as _,
	)
	.fetch_all(&mut *database)
	.await?
	.into_iter()
	.map(|record| (record.deployment_id, record.port, record.host));

	debug!("Updating tunnel configuration to handle the runner");

	client
		.put(format!(
			"{}accounts/{}/cfd_tunnel/{}/configurations",
			config.cloudflare.base_url, config.cloudflare.account_id, tunnel.id
		))
		.bearer_auth(&config.cloudflare.api_key)
		.json(&TunnelConfigRequest {
			config: TunnelConfigRequestConfig {
				ingress: deployment_ports
					.chain(managed_url_ports)
					.map(
						|(deployment_id, port, host)| TunnelConfigRequestConfigIngress {
							hostname: Some(host),
							service: format!("http://{}.onpatr.local:{}", deployment_id, port),
						},
					)
					.chain(std::iter::once(TunnelConfigRequestConfigIngress {
						hostname: None,
						service: "http_status:404".to_string(),
					}))
					.collect(),
			},
		})
		.send()
		.await?
		.error_for_status()?;

	Ok(())
}
