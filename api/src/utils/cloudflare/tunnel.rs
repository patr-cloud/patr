use cloudflare::{
	endpoints::cfd_tunnel::{ConfigurationSrc, create_tunnel},
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
use serde::Serialize;

use crate::{prelude::*, utils::config::AppConfig};

/// Cloudflare tunnel configuration request body
#[derive(Serialize)]
struct TunnelConfigRequest {
	config: TunnelConfigRequestConfig,
}

/// Ingress rules for the tunnel
#[derive(Serialize)]
struct TunnelConfigRequestConfig {
	ingress: Vec<TunnelConfigRequestConfigIngress>,
}

/// A single ingress rule
#[derive(Serialize)]
struct TunnelConfigRequestConfigIngress {
	/// The hostname to match. `None` makes it a catch-all.
	#[serde(skip_serializing_if = "Option::is_none")]
	hostname: Option<String>,
	/// The service to route to.
	service: String,
}

/// Create a new Cloudflare Tunnel for a runner and set a static catch-all
/// ingress rule routing all traffic through Caddy. Returns the tunnel ID.
pub async fn create_tunnel_with_config(
	runner_id: Uuid,
	config: &AppConfig,
) -> Result<String, ErrorType> {
	let tunnel = CloudflareClient::new(
		Credentials::UserAuthToken {
			token: config.cloudflare.api_key.clone(),
		},
		ClientConfig::default(),
		Environment::Custom(config.cloudflare.base_url.clone()),
	)?
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
	.result;

	let tunnel_id = tunnel.id.to_string();

	// Set a static catch-all ingress rule. DNS routing is handled by the
	// Cloudflare Worker, so this single rule is all that's needed.
	reqwest::Client::new()
		.put(format!(
			"{}accounts/{}/cfd_tunnel/{}/configurations",
			config.cloudflare.base_url, config.cloudflare.account_id, tunnel_id
		))
		.bearer_auth(&config.cloudflare.api_key)
		.json(&TunnelConfigRequest {
			config: TunnelConfigRequestConfig {
				ingress: vec![TunnelConfigRequestConfigIngress {
					hostname: None,
					service: String::from("http://patr-ingress:80"),
				}],
			},
		})
		.send()
		.await?
		.error_for_status()?;

	Ok(tunnel_id)
}
