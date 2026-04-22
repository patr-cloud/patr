use std::collections::HashMap;

use bollard::{Docker, models::ConfigSpec, query_parameters::ListConfigsOptions};
use sha2::{Digest as _, Sha256};

use crate::prelude::*;

/// Create or reuse a Docker config using content-hash naming.
///
/// Docker Swarm configs are immutable — data cannot be updated after creation.
/// This function works around that by including a hash of the data in the
/// config name: `{base_name}-{sha256(data)[:16]}`. The full hash is stored in a
/// `patr.configHash` label for reliable matching.
///
/// Returns `(config_id, config_name)`. If a config with the same hash already
/// exists, it is reused (no-op). Otherwise, a new config is created and old
/// configs with the same labels are cleaned up.
pub async fn update_config(
	docker: &Docker,
	base_name: &str,
	mut labels: HashMap<String, String>,
	data: String,
) -> Result<(String, String), RunnerError> {
	let full_hash = Sha256::digest(&data)
		.iter()
		.map(|byte| format!("{:02x}", byte))
		.collect::<String>();

	// Scope list/cleanup to configs with the same base_name. Without this,
	// callers that share labels across multiple logical configs (e.g. one per
	// deployment mount ordinal) would have cleanup trample siblings.
	labels.insert(String::from("patr.configBaseName"), String::from(base_name));

	// List existing configs that share the same labels (excluding the hash label)
	let label_filter = labels
		.iter()
		.map(|(k, v)| format!("{}={}", k, v))
		.collect::<Vec<_>>();

	let existing_configs = docker
		.list_configs(Some(ListConfigsOptions {
			filters: Some(HashMap::from([(String::from("label"), label_filter)])),
		}))
		.await
		.map_err(RunnerError::host)?;

	// Pick the shortest hash suffix (starting at 16) that doesn't collide with
	// an existing config that has different data.
	let mut hash_len = 16;
	let config_name = loop {
		let candidate = format!(
			"{}-{}",
			base_name,
			full_hash.chars().take(hash_len).collect::<String>()
		);
		let collision = existing_configs.iter().any(|c| {
			let same_name = c
				.spec
				.as_ref()
				.and_then(|s| s.name.as_ref())
				.is_some_and(|n| n == &candidate);
			let different_hash = c
				.spec
				.as_ref()
				.and_then(|s| s.labels.as_ref())
				.and_then(|l| l.get("patr.configHash"))
				.is_some_and(|h| h != &full_hash);
			same_name && different_hash
		});
		if !collision || hash_len == full_hash.len() {
			break candidate;
		}
		hash_len += 1;
	};

	// Check if a config with the same hash already exists
	// If it does, reuse it (no-op). This handles the common case of unchanged
	// configs without unnecessary churn
	for config in &existing_configs {
		let hash_matches = config
			.spec
			.as_ref()
			.and_then(|spec| spec.labels.as_ref())
			.and_then(|labels| labels.get("patr.configHash"))
			.is_some_and(|h| h == &full_hash);

		if hash_matches && let Some(id) = &config.id {
			let name = config
				.spec
				.as_ref()
				.and_then(|s| s.name.clone())
				.unwrap_or_else(|| config_name.clone());
			return Ok((id.clone(), name));
		}
	}

	// Data changed (or first creation) — create new config with hashed name
	labels.insert(String::from("patr.configHash"), full_hash);
	labels.insert(
		String::from("patr.version"),
		String::from(constants::PATR_VERSION),
	);

	let new_id = docker
		.create_config(ConfigSpec {
			name: Some(config_name.clone()),
			labels: Some(labels),
			data: Some(data),
			templating: None,
		})
		.await
		.map_err(RunnerError::host)?
		.id;

	// Clean up old configs with the same base_name but different content hashes.
	// The label filter above already scopes to this base_name family, so there's
	// no risk of deleting configs owned by other callers.
	for config in existing_configs {
		if let Some(id) = config.id &&
			id != new_id &&
			let Err(err) = docker.delete_config(&id).await
		{
			warn!("Failed to clean up old config {}: {}", id, err);
		}
	}

	Ok((new_id, config_name))
}

/// All commonly used constants in the Docker runner.
pub mod constants {
	/// The current crate version, stamped onto every `managed-by=patr` Swarm
	/// resource via the `patr.version` label.
	pub const PATR_VERSION: &str = env!("CARGO_PKG_VERSION");
	/// The name of the patr overlay network for service discovery.
	/// NOTE: This must NOT be "ingress" - that's Docker Swarm's built-in
	/// routing mesh network which does not support DNS-based service discovery.
	pub const INGRESS_NETWORK_NAME: &str = "patr-ingress-network";
	/// The name of the ingress service.
	pub const INGRESS_SERVICE_NAME: &str = "patr-ingress";
	/// The name of the volume used to store TLS certs for the ingress service.
	pub const INGRESS_TLS_CERTS_VOLUME_NAME: &str = "patr-ingress-data";
	/// The name of the Cloudflare tunnel service (private runners only).
	pub const TUNNEL_SERVICE_NAME: &str = "patr-tunnel";
	/// The name of the config used to store the cloudflare tunnel token.
	pub const TUNNEL_TOKEN_CONFIG_NAME: &str = "patr-tunnel-token";
	/// The name of the config used to store the ingress configuration for the
	/// runner.
	pub const INGRESS_CONFIG_NAME: &str = "patr-ingress-config";
	/// The name of the Grafana Alloy log collector service.
	pub const ALLOY_SERVICE_NAME: &str = "patr-alloy";
	/// The name of the Docker config for the Alloy configuration.
	pub const ALLOY_CONFIG_NAME: &str = "patr-alloy-config";
	/// The pinned Grafana Alloy image version.
	pub const ALLOY_IMAGE: &str = "grafana/alloy:v1.13.2";
}
