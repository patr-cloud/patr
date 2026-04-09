use cloudflare::{
	endpoints::cfd_tunnel::*,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
		response::ApiSuccess,
	},
};
use serde::{Deserialize, Serialize};

use crate::{prelude::*, utils::config::AppConfig};

/// The configuration for the tunnel
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TunnelConfigRequest {
	config: TunnelConfigRequestConfig,
}

/// The list of ingress rules for the tunnel
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TunnelConfigRequestConfig {
	ingress: Vec<TunnelConfigRequestConfigIngress>,
}

/// An ingress rule for the tunnel
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TunnelConfigRequestConfigIngress {
	/// The hostname for the ingress rule. `None` makes it a catch-all.
	hostname: Option<String>,
	/// The service to route to.
	service: String,
}

/// Sync the Cloudflare Tunnel configuration for a runner. Queries all
/// deployments running on the runner and their managed URLs, then updates
/// the tunnel ingress rules accordingly.
pub async fn update_tunnel_config_for_runner(
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
	let cf_client = CloudflareClient::new(
		Credentials::UserAuthToken {
			token: config.cloudflare.api_key.clone(),
		},
		ClientConfig::default(),
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
