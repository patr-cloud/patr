use std::collections::HashMap;

use bollard::{
	Docker,
	models::{Mount, MountTypeEnum},
	query_parameters::UpdateServiceOptionsBuilder,
	service::{
		NetworkAttachmentConfig,
		ServiceSpec,
		ServiceSpecMode,
		TaskSpec,
		TaskSpecContainerSpec,
		TaskSpecContainerSpecConfigs,
		TaskSpecContainerSpecFile1,
	},
};

use crate::prelude::*;

/// Ensure the Grafana Alloy log collector service is running with the latest
/// config. Creates or updates the Alloy Docker config and global Swarm service.
///
/// This should only be called in managed mode — the caller is responsible for
/// checking the runner mode before calling this function.
pub async fn update_alloy_service(
	docker: &Docker,
	settings: &RunnerSettings<DockerSettings>,
) -> Result<(), RunnerError> {
	let RunnerMode::Managed {
		workspace_id,
		runner_id,
		api_token,
		..
	} = &settings.mode
	else {
		return Ok(());
	};

	let loki_url = match settings.environment {
		RunningEnvironment::Production => "https://loki.patr.cloud",
		RunningEnvironment::Development => "http://host.docker.internal:3003",
	};

	let mimir_url = match settings.environment {
		RunningEnvironment::Production => "https://mimir.patr.cloud",
		RunningEnvironment::Development => "http://host.docker.internal:3005",
	};

	let alloy_config_text =
		generate_alloy_config(workspace_id, runner_id, api_token, loki_url, mimir_url);

	// Create or reuse the Docker config for Alloy (content-hash naming)
	let (alloy_config_id, alloy_config_name) = crate::utils::update_config(
		docker,
		constants::ALLOY_CONFIG_NAME,
		HashMap::from([(String::from("managed-by"), String::from("patr"))]),
		Base64String::from_string(alloy_config_text).to_string(),
	)
	.await?;

	// Build the global Alloy service spec
	let service_spec = ServiceSpec {
		name: Some(String::from(constants::ALLOY_SERVICE_NAME)),
		labels: Some(HashMap::from([(
			String::from("managed-by"),
			String::from("patr"),
		)])),
		task_template: Some(TaskSpec {
			container_spec: Some(TaskSpecContainerSpec {
				image: Some(String::from(constants::ALLOY_IMAGE)),
				labels: Some(HashMap::from([(
					String::from("managed-by"),
					String::from("patr"),
				)])),
				command: Some(vec![
					String::from("alloy"),
					String::from("run"),
					String::from("/etc/alloy/config.alloy"),
				]),
				configs: Some(vec![TaskSpecContainerSpecConfigs {
					file: Some(TaskSpecContainerSpecFile1 {
						name: Some(String::from("/etc/alloy/config.alloy")),
						mode: Some(0o444),
						uid: Some(String::from("0")),
						gid: Some(String::from("0")),
					}),
					config_id: Some(alloy_config_id),
					config_name: Some(alloy_config_name),
					runtime: None,
				}]),
				mounts: Some(vec![
					Mount {
						target: Some(String::from("/var/run/docker.sock")),
						source: Some(String::from("/var/run/docker.sock")),
						typ: Some(MountTypeEnum::BIND),
						read_only: Some(true),
						..Default::default()
					},
					Mount {
						target: Some(String::from("/host/proc")),
						source: Some(String::from("/proc")),
						typ: Some(MountTypeEnum::BIND),
						read_only: Some(true),
						..Default::default()
					},
					Mount {
						target: Some(String::from("/host/sys")),
						source: Some(String::from("/sys")),
						typ: Some(MountTypeEnum::BIND),
						read_only: Some(true),
						..Default::default()
					},
					Mount {
						target: Some(String::from("/host/root")),
						source: Some(String::from("/")),
						typ: Some(MountTypeEnum::BIND),
						read_only: Some(true),
						..Default::default()
					},
				]),
				..Default::default()
			}),
			..Default::default()
		}),
		mode: Some(ServiceSpecMode {
			global: Some(HashMap::new()),
			..Default::default()
		}),
		networks: Some(vec![NetworkAttachmentConfig {
			target: Some(String::from(constants::INGRESS_NETWORK_NAME)),
			aliases: Some(vec![String::from("patr-alloy")]),
			driver_opts: None,
		}]),
		..Default::default()
	};

	// Create or update the Alloy service
	let alloy_service = docker
		.inspect_service(constants::ALLOY_SERVICE_NAME, None)
		.await
		.ok();

	if let Some(version) = alloy_service
		.and_then(|service| service.version)
		.and_then(|version| version.index)
	{
		docker
			.update_service(
				constants::ALLOY_SERVICE_NAME,
				service_spec,
				UpdateServiceOptionsBuilder::new()
					.version(version as i32)
					.build(),
				None,
			)
			.await
			.map_err(RunnerError::host)?;
	} else {
		docker
			.create_service(service_spec, None)
			.await
			.map_err(RunnerError::host)?;
	}

	Ok(())
}

/// Generate the Alloy configuration string with interpolated values.
fn generate_alloy_config(
	workspace_id: &Uuid,
	runner_id: &Uuid,
	api_token: &BearerToken,
	loki_url: &str,
	mimir_url: &str,
) -> String {
	format!(
		r#"
discovery.docker "patr" {{
  host = "unix:///var/run/docker.sock"
  filter {{
    name = "label"
    values = ["patr.deploymentId"]
  }}
}}

discovery.relabel "patr" {{
  targets = discovery.docker.patr.targets

  rule {{
    source_labels = ["__meta_docker_container_label_patr_deploymentId"]
    target_label  = "deployment_id"
  }}
  rule {{
    source_labels = ["__meta_docker_container_label_patr_deploymentName"]
    target_label  = "deployment_name"
  }}
  rule {{
    target_label = "runner_id"
    replacement  = "{runner_id}"
  }}
  rule {{
    target_label = "workspace_id"
    replacement  = "{workspace_id}"
  }}
  rule {{
    target_label = "source"
    replacement  = "deployment"
  }}
}}

loki.source.docker "patr" {{
  host       = "unix:///var/run/docker.sock"
  targets    = discovery.relabel.patr.output
  forward_to = [loki.write.patr.receiver]
}}

loki.write "patr" {{
  endpoint {{
    url = "{loki_url}/loki/api/v1/push"
    batch_size = "4MiB"

    basic_auth {{
      username = "{runner_id}"
      password = "{api_token}"
    }}
  }}
}}

prometheus.exporter.unix "system" {{
  rootfs_path = "/host/root"
  procfs_path = "/host/proc"
  sysfs_path  = "/host/sys"
}}

prometheus.scrape "system" {{
  targets    = prometheus.exporter.unix.system.targets
  forward_to = [prometheus.relabel.system.receiver]
}}

prometheus.relabel "system" {{
  forward_to = [prometheus.remote_write.mimir.receiver]

  rule {{
    target_label = "runner_id"
    replacement  = "{runner_id}"
  }}
  rule {{
    target_label = "workspace_id"
    replacement  = "{workspace_id}"
  }}
  rule {{
    target_label = "source"
    replacement  = "runner"
  }}
}}

prometheus.remote_write "mimir" {{
  endpoint {{
    url = "{mimir_url}/api/v1/push"

    basic_auth {{
      username = "{runner_id}"
      password = "{api_token}"
    }}
  }}
}}
"#,
		runner_id = runner_id,
		workspace_id = workspace_id,
		loki_url = loki_url,
		mimir_url = mimir_url,
		api_token = api_token.0.token(),
	)
}
