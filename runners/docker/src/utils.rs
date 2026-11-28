use std::collections::HashMap;

use bollard::{
	Docker,
	auth::DockerCredentials,
	errors::Error as DockerError,
	models::ServiceSpec,
	query_parameters::UpdateServiceOptionsBuilder,
};
use sha2::{Digest as _, Sha256};

use crate::prelude::*;

/// The labels every object the runner applies carries: who manages it, its
/// owner, the hash of the spec it was applied with and the runner version.
pub fn object_labels(owner: Owner, hash: &str) -> HashMap<String, String> {
	HashMap::from([
		(String::from("managed-by"), String::from("patr")),
		(String::from("patr.owner"), owner.to_string()),
		(String::from("patr.specHash"), String::from(hash)),
		(
			String::from("patr.version"),
			String::from(constants::PATR_VERSION),
		),
	])
}

/// The lowercase hex SHA-256 of `data`.
pub fn sha256_hex(data: impl AsRef<[u8]>) -> String {
	Sha256::digest(data.as_ref())
		.iter()
		.map(|byte| format!("{byte:02x}"))
		.collect::<String>()
}

/// The name of a config holding `data`: `{base}-{sha256(data)[..16]}`. Swarm
/// configs are immutable, so new content gets a new config, and the old one is
/// reaped once nothing uses it.
pub fn config_name(base: &str, data: impl AsRef<[u8]>) -> String {
	format!("{base}-{}", &sha256_hex(data)[..16])
}

/// The Docker volume name backing a deployment volume:
/// `patr-{deployment_id}-{sha256(path)[..16]}`.
///
/// Derived from the *path*, never an ordinal. Config mounts can use ordinals
/// safely because they're stateless; for a volume an ordinal shift would remap
/// a directory's data onto a different directory. Hashing the path also means
/// removing a volume and re-adding the same path finds the same data.
pub fn deployment_volume_name(deployment_id: Uuid, path: &str) -> String {
	format!("patr-{deployment_id}-{}", &sha256_hex(path)[..16])
}

/// Creates a service, or updates it if it exists. Swarm rejects an update
/// whose version isn't the service's current one, so on that conflict the
/// version is read again and the update retried once.
pub async fn upsert_service(
	docker: &Docker,
	spec: ServiceSpec,
	registry_auth: Option<DockerCredentials>,
) -> Result<(), RunnerError> {
	let name = spec.name.clone().unwrap_or_default();
	let mut retried = false;
	loop {
		let version = match docker.inspect_service(&name, None).await {
			Ok(service) => service
				.version
				.and_then(|version| version.index)
				.unwrap_or(0),
			Err(DockerError::DockerResponseServerError {
				status_code: 404, ..
			}) => {
				docker
					.create_service(spec, registry_auth)
					.await
					.map_err(RunnerError::host)?;
				return Ok(());
			}
			Err(err) => return Err(RunnerError::host(err)),
		};

		match docker
			.update_service(
				&name,
				spec.clone(),
				UpdateServiceOptionsBuilder::new()
					.version(i32::try_from(version).map_err(RunnerError::host)?)
					.build(),
				registry_auth.clone(),
			)
			.await
		{
			Ok(_) => return Ok(()),
			Err(DockerError::DockerResponseServerError { message, .. })
				if !retried && message.contains("out of sequence") =>
			{
				retried = true;
			}
			Err(err) => return Err(RunnerError::host(err)),
		}
	}
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
	/// The base name of the config holding the ingress's main Caddyfile.
	pub const INGRESS_CONFIG_NAME: &str = "patr-ingress-config";
	/// The name of the Grafana Alloy log collector service.
	pub const ALLOY_SERVICE_NAME: &str = "patr-alloy";
	/// The base name of the config holding the Alloy configuration.
	pub const ALLOY_CONFIG_NAME: &str = "patr-alloy-config";
	/// The pinned Grafana Alloy image version.
	pub const ALLOY_IMAGE: &str = "grafana/alloy:v1.13.2";
}
