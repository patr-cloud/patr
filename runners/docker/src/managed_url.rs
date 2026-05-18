//! Managed URL → Caddy config bridge for the Docker runner.
//!
//! Each managed URL is materialized as a Docker Swarm config labeled
//! `patr.configBaseName=managed-url-{id}`. The ingress builder picks these up
//! alongside the per-deployment configs and mounts them into Caddy.

use std::collections::HashMap;

use bollard::query_parameters::{ListConfigsOptions, UpdateServiceOptionsBuilder};

use crate::prelude::*;

/// Generate the Caddyfile snippet for a managed URL.
///
/// `is_private` corresponds to runners fronted by a Cloudflare Tunnel — TLS
/// terminates at Cloudflare and `cloudflared` forwards to Caddy as HTTP. The
/// site address gets an explicit `http://` prefix so Caddy doesn't auto-enable
/// HTTPS and generate an HTTP→HTTPS redirect that would loop back through
/// `cloudflared`. Public runners terminate TLS at Caddy itself, so we leave
/// the scheme blank and let auto-HTTPS do its thing.
pub(crate) fn generate_config(
	host: &str,
	path: &str,
	deployment_id: Uuid,
	port: u16,
	is_private: bool,
) -> String {
	format!(
		include_str!("../../../assets/runner/Caddyfile.managed-url.template"),
		scheme = if is_private { "http://" } else { "" },
		host = host,
		path = path,
		deployment_id = deployment_id,
		port = port,
	)
}

/// Create or update the Docker config for a managed URL, then trigger an
/// ingress refresh. Mirrors the deployment upsert flow in `ingress.rs`.
pub(crate) async fn upsert(
	runner: &DockerRunner,
	managed_url_id: Uuid,
	host: String,
	path: String,
	deployment_id: Uuid,
	port: u16,
) -> Result<(), RunnerError> {
	let config = generate_config(
		&host,
		&path,
		deployment_id,
		port,
		runner.settings.data.runner_exposure_type.is_private(),
	);

	let labels = HashMap::from([
		(String::from("managed-by"), String::from("patr")),
		(
			String::from("patr.managedUrlId"),
			managed_url_id.to_string(),
		),
	]);

	crate::utils::update_config(
		&runner.docker,
		&format!("managed-url-{}", managed_url_id),
		labels,
		config,
	)
	.await?;

	let _guard = runner.ingress_lock.lock().await;
	ingress::update_ingress_configs(&runner.docker, &runner.settings).await
}

/// Delete the Docker config for a managed URL and rebuild the ingress
/// service so Caddy stops serving it. Mirrors `delete_deployment_config`.
pub(crate) async fn delete(runner: &DockerRunner, managed_url_id: Uuid) -> Result<(), RunnerError> {
	let configs = runner
		.docker
		.list_configs(Some(ListConfigsOptions {
			filters: Some(HashMap::from([(
				String::from("label"),
				vec![format!("patr.managedUrlId={}", managed_url_id)],
			)])),
		}))
		.await
		.map_err(RunnerError::host)?;

	let config_ids_to_remove = configs
		.iter()
		.filter_map(|c| c.id.clone())
		.collect::<Vec<_>>();

	let _guard = runner.ingress_lock.lock().await;

	// Rebuild the ingress spec without these configs first, then delete them
	// so we never leave a "config in use" gap.
	let mut ingress_service_spec =
		ingress::build_ingress_spec(&runner.docker, &runner.settings).await?;
	if let Some(spec_configs) = ingress_service_spec
		.task_template
		.as_mut()
		.and_then(|task| task.container_spec.as_mut())
		.and_then(|container| container.configs.as_mut())
	{
		spec_configs.retain(|c| {
			c.config_id
				.as_ref()
				.is_none_or(|id| !config_ids_to_remove.contains(id))
		});
	}

	if let Some(version) = runner
		.docker
		.inspect_service(constants::INGRESS_SERVICE_NAME, None)
		.await
		.ok()
		.and_then(|s| s.version)
		.and_then(|v| v.index)
	{
		runner
			.docker
			.update_service(
				constants::INGRESS_SERVICE_NAME,
				ingress_service_spec,
				UpdateServiceOptionsBuilder::new()
					.version(version as i32)
					.build(),
				None,
			)
			.await
			.map_err(RunnerError::host)?;
	}

	for config in configs {
		if let Some(id) = config.id {
			runner.docker.delete_config(&id).await.map_err(|err| {
				error!("Failed to delete managed URL config {}: {}", id, err);
				RunnerError::host(err)
			})?;
		}
	}

	Ok(())
}

/// List the IDs of all managed URLs that currently have a Docker config on
/// this runner.
pub(crate) async fn list_running(runner: &DockerRunner) -> Result<Vec<Uuid>, RunnerError> {
	let configs = runner
		.docker
		.list_configs(Some(ListConfigsOptions {
			filters: Some(HashMap::from([(
				String::from("label"),
				vec![String::from("patr.managedUrlId")],
			)])),
		}))
		.await
		.map_err(RunnerError::host)?;

	let mut ids = configs
		.into_iter()
		.filter_map(|c| {
			c.spec
				.as_ref()?
				.labels
				.as_ref()?
				.get("patr.managedUrlId")?
				.parse::<Uuid>()
				.ok()
		})
		.collect::<Vec<_>>();
	ids.sort();
	ids.dedup();
	Ok(ids)
}
