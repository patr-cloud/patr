use std::{cmp::Ordering, collections::BTreeMap, str};

use api_models::{
	models::workspace::{
		infrastructure::deployment::{
			Deployment,
			DeploymentLogs,
			DeploymentMetrics,
			DeploymentProbe,
			DeploymentRegistry,
			DeploymentRunningDetails,
			DeploymentStatus,
			DeploymentVolume,
			EnvironmentVariableValue,
			ExposedPortType,
			Metric,
			PatrRegistry,
		},
		region::RegionStatus,
	},
	utils::{
		constants,
		Base64String,
		DateTime as TzDateTime,
		StringifiedU16,
		Uuid,
	},
};
use chrono::{DateTime, TimeZone, Utc};
use eve_rs::AsError;
use k8s_openapi::api::core::v1::Event;
use reqwest::Client;

use crate::{
	db,
	error,
	models::{
		cloudflare,
		deployment::{Logs, PrometheusResponse, MACHINE_TYPES},
		rbac::{self, permissions},
		DeploymentMetadata,
	},
	service,
	utils::{constants::free_limits, settings::Settings, validator, Error},
	Database,
};

/// # Description
/// This function creates a deployment under an workspace account
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `workspace_id` -  an unsigned 8 bit integer array containing the id of
///   workspace
/// * `name` - a string containing the name of deployment
/// * `registry` - a string containing the url of docker registry
/// * `repository_id` - An Option<&str> containing either a repository id of
///   type string or `None`
/// * `image_name` - An Option<&str> containing either an image name of type
///   string or `None`
/// * `image_tag` - a string containing tags of docker image
///
/// # Returns
/// This function returns Result<Uuid, Error> containing an uuid of the
/// deployment or an error
///
/// [`Transaction`]: Transaction
pub async fn create_deployment_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	name: &str,
	registry: &DeploymentRegistry,
	image_tag: &str,
	region: &Uuid,
	machine_type: &Uuid,
	deployment_running_details: &DeploymentRunningDetails,
	request_id: &Uuid,
) -> Result<Uuid, Error> {
	if image_tag.is_empty() {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	if let DeploymentRegistry::ExternalRegistry { image_name, .. } = registry {
		if !validator::is_docker_image_name_valid(image_name) {
			log::trace!(
				"request_id: {} invalid image_name cannot contain colon(:)",
				request_id
			);
			return Err(Error::empty()
				.status(400)
				.body(error!(INVALID_IMAGE_NAME).to_string()));
		}
	}

	// validate deployment name
	log::trace!("request_id: {} - Validating deployment name", request_id);
	if !validator::is_deployment_name_valid(name) {
		return Err(Error::empty()
			.status(200)
			.body(error!(INVALID_DEPLOYMENT_NAME).to_string()));
	}

	// validate whether the deployment region is ready
	let region_details = db::get_region_by_id(connection, region)
		.await?
		.filter(|value| {
			value.is_patr_region() ||
				value.workspace_id.as_ref() == Some(workspace_id)
		})
		.status(400)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	if !(region_details.status == RegionStatus::Active ||
		region_details.is_patr_region())
	{
		return Err(Error::empty()
			.status(500)
			.body(error!(REGION_NOT_READY_YET).to_string()));
	}

	log::trace!(
		"request_id: {} - Checking if the deployment name already exists",
		request_id
	);
	let existing_deployment =
		db::get_deployment_by_name_in_workspace(connection, name, workspace_id)
			.await?;
	if existing_deployment.is_some() {
		Error::as_result()
			.status(200)
			.body(error!(RESOURCE_EXISTS).to_string())?;
	}

	log::trace!("request_id: {} - Generating new resource id", request_id);
	let deployment_id = db::generate_new_resource_id(connection).await?;

	if deployment_running_details.ports.is_empty() {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	check_deployment_creation_limit(
		connection,
		workspace_id,
		region_details.is_byoc_region(),
		machine_type,
		&deployment_running_details.min_horizontal_scale,
		&deployment_running_details.max_horizontal_scale,
		&deployment_running_details.volumes,
		request_id,
	)
	.await?;

	let created_time = Utc::now();

	db::create_resource(
		connection,
		&deployment_id,
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::DEPLOYMENT)
			.unwrap(),
		workspace_id,
		&created_time,
	)
	.await?;
	log::trace!("request_id: {} - Created resource", request_id);

	db::begin_deferred_constraints(connection).await?;
	match registry {
		DeploymentRegistry::PatrRegistry {
			registry: _,
			repository_id,
		} => {
			log::trace!("request_id: {} - Creating database record with internal registry", request_id);
			db::create_deployment_with_internal_registry(
				connection,
				&deployment_id,
				name,
				repository_id,
				image_tag,
				workspace_id,
				region,
				machine_type,
				deployment_running_details.deploy_on_push,
				deployment_running_details.min_horizontal_scale,
				deployment_running_details.max_horizontal_scale,
				deployment_running_details.startup_probe.as_ref(),
				deployment_running_details.liveness_probe.as_ref(),
			)
			.await?;
		}
		DeploymentRegistry::ExternalRegistry {
			registry,
			image_name,
		} => {
			log::trace!("request_id: {} - Creating database record with external registry", request_id);
			db::create_deployment_with_external_registry(
				connection,
				&deployment_id,
				name,
				registry,
				image_name,
				image_tag,
				workspace_id,
				region,
				machine_type,
				deployment_running_details.deploy_on_push,
				deployment_running_details.min_horizontal_scale,
				deployment_running_details.max_horizontal_scale,
				deployment_running_details.startup_probe.as_ref(),
				deployment_running_details.liveness_probe.as_ref(),
			)
			.await?;
		}
	}

	for (port, port_type) in &deployment_running_details.ports {
		log::trace!(
			"request_id: {} - Adding exposed port entry to database",
			request_id
		);
		db::add_exposed_port_for_deployment(
			connection,
			&deployment_id,
			port.value(),
			port_type,
		)
		.await?;
	}
	db::end_deferred_constraints(connection).await?;

	for (key, value) in &deployment_running_details.environment_variables {
		log::trace!(
			"request_id: {} - Adding environment variable entry to database",
			request_id
		);

		match value {
			EnvironmentVariableValue::String(value) => {
				db::add_environment_variable_for_deployment(
					connection,
					&deployment_id,
					key,
					Some(value),
					None,
				)
			}
			EnvironmentVariableValue::Secret {
				from_secret: secret_id,
			} => db::add_environment_variable_for_deployment(
				connection,
				&deployment_id,
				key,
				None,
				Some(secret_id),
			),
		}
		.await?;
	}

	for (path, file) in &deployment_running_details.config_mounts {
		log::trace!(
			"request_id: {} - Decoding config file from base64 to byte array",
			request_id
		);
		db::add_config_mount_for_deployment(
			connection,
			&deployment_id,
			path.as_ref(),
			file,
		)
		.await?;
	}

	for (name, volume) in &deployment_running_details.volumes {
		log::trace!("request_id: {} - creating volume resource", request_id);
		let volume_id = db::generate_new_resource_id(connection).await?;

		db::create_resource(
			connection,
			&volume_id,
			rbac::RESOURCE_TYPES
				.get()
				.unwrap()
				.get(rbac::resource_types::DEPLOYMENT_VOLUME)
				.unwrap(),
			workspace_id,
			&created_time,
		)
		.await?;

		db::add_volume_for_deployment(
			connection,
			&deployment_id,
			&volume_id,
			name.as_str(),
			volume.size as i32,
			volume.path.as_str(),
		)
		.await?;
	}

	Ok(deployment_id)
}

pub async fn get_deployment_container_logs(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	start_time: &DateTime<Utc>,
	end_time: &DateTime<Utc>,
	limit: u32,
	config: &Settings,
	request_id: &Uuid,
) -> Result<Vec<DeploymentLogs>, Error> {
	log::trace!(
		"Getting deployment logs for deployment_id: {} with request_id: {}",
		deployment_id,
		request_id
	);

	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let logs = get_container_logs(
		&deployment.workspace_id,
		deployment_id,
		start_time,
		end_time,
		limit,
		config,
		request_id,
	)
	.await?;

	log::trace!("request_id: {} - Logs retreived successfully", request_id);

	Ok(logs)
}

#[allow(clippy::too_many_arguments)]
pub async fn update_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	name: Option<&str>,
	machine_type: Option<&Uuid>,
	deploy_on_push: Option<bool>,
	min_horizontal_scale: Option<u16>,
	max_horizontal_scale: Option<u16>,
	ports: Option<&BTreeMap<u16, ExposedPortType>>,
	environment_variables: Option<&BTreeMap<String, EnvironmentVariableValue>>,
	startup_probe: Option<&DeploymentProbe>,
	liveness_probe: Option<&DeploymentProbe>,
	config_mounts: Option<&BTreeMap<String, Base64String>>,
	volumes: Option<&BTreeMap<String, DeploymentVolume>>,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Updating deployment with id: {}",
		request_id,
		deployment_id
	);

	let now = Utc::now();

	let old_min_replicas = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?
		.min_horizontal_scale as u16;

	// Get volume size for checking the limit
	let volume_size = if let Some(volumes) = volumes {
		volumes
			.iter()
			.map(|(_, volume)| volume.size as u32)
			.sum::<u32>()
	} else {
		0
	};

	// Check if card is added
	let card_added =
		db::get_default_payment_method_for_workspace(connection, workspace_id)
			.await?
			.is_some();
	if !card_added {
		if let Some(machine_type) = machine_type {
			// only basic machine type is allowed under free plan
			let machine_type_to_be_deployed = MACHINE_TYPES
				.get()
				.and_then(|machines| machines.get(machine_type))
				.status(500)?;

			if machine_type_to_be_deployed != &(1, 2) {
				log::info!("request_id: {request_id} - Only basic machine type is allowed
	under free plan");
				return Error::as_result().status(400).body(
					error!(CARDLESS_DEPLOYMENT_MACHINE_TYPE_LIMIT).to_string(),
				)?;
			}
		}
		if let Some(max_horizontal_scale) = max_horizontal_scale {
			if max_horizontal_scale > 1 {
				log::info!(
					"request_id: {request_id} - Only one replica allowed under free plan
	without card" 			);
				return Error::as_result()
					.status(400)
					.body(error!(REPLICA_LIMIT_EXCEEDED).to_string())?;
			}
		}

		if let Some(min_horizontal_scale) = min_horizontal_scale {
			if min_horizontal_scale > 1 {
				log::info!(
					"request_id: {request_id} - Only one replica allowed under free plan
	without card" 			);
				return Error::as_result()
					.status(400)
					.body(error!(REPLICA_LIMIT_EXCEEDED).to_string())?;
			}
		}

		let volume_size_in_bytes = volume_size as usize * 1024 * 1024 * 1024;
		if volume_size_in_bytes > free_limits::VOLUME_STORAGE_IN_BYTE {
			return Error::as_result()
				.status(400)
				.body(error!(CARDLESS_VOLUME_LIMIT_EXCEEDED).to_string())?;
		}
	}

	let workspace = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)?;
	let workspace_volume_limit = workspace.volume_storage_limit;

	if volume_size > workspace_volume_limit as u32 {
		return Error::as_result()
			.status(400)
			.body(error!(VOLUME_LIMIT_EXCEEDED).to_string())?;
	}

	if workspace.is_spam {
		return Err(Error::empty()
			.status(401)
			.body(error!(UNVERIFIED_WORKSPACE).to_string()));
	}

	db::begin_deferred_constraints(connection).await?;

	if let Some(ports) = ports {
		if ports.is_empty() {
			return Err(Error::empty()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string()));
		}

		log::trace!(
			"request_id: {} - Updating deployment ports in the database",
			request_id
		);
		db::remove_all_exposed_ports_for_deployment(connection, deployment_id)
			.await?;
		for (port, exposed_port_type) in ports {
			db::add_exposed_port_for_deployment(
				connection,
				deployment_id,
				*port,
				exposed_port_type,
			)
			.await?;
		}
	}

	db::update_deployment_details(
		connection,
		deployment_id,
		name,
		machine_type,
		deploy_on_push,
		min_horizontal_scale,
		max_horizontal_scale,
		startup_probe,
		liveness_probe,
	)
	.await?;

	db::end_deferred_constraints(connection).await?;

	if let Some(config_mounts) = config_mounts {
		db::remove_all_config_mounts_for_deployment(connection, deployment_id)
			.await?;
		for (path, file) in config_mounts {
			db::add_config_mount_for_deployment(
				connection,
				deployment_id,
				path.as_ref(),
				file,
			)
			.await?;
		}
	}

	if let Some(updated_volumes) = volumes {
		let mut current_volumes =
			db::get_all_deployment_volumes(connection, deployment_id)
				.await?
				.into_iter()
				.map(|volume| (volume.name.clone(), volume))
				.collect::<BTreeMap<_, _>>();

		for (name, volume) in updated_volumes {
			if let Some(value) = current_volumes.remove(name) {
				// The new volume is there in the current volumes. Update it

				let current_size = value.size as u16;
				let new_size = volume.size;

				match new_size.cmp(&current_size) {
					Ordering::Less => {
						// Volume size cannot be reduced
						return Err(Error::empty())
							.status(400)
							.body(error!(REDUCED_VOLUME_SIZE).to_string());
					}
					Ordering::Equal => (), // Ignore
					Ordering::Greater => {
						db::update_volume_for_deployment(
							connection,
							deployment_id,
							new_size as i32,
							name,
						)
						.await?;
					}
				}
			} else {
				// The new volume is not there in the current volumes. Prevent
				// from adding it
				return Err(Error::empty())
					.status(400)
					.body(error!(CANNOT_ADD_NEW_VOLUME).to_string());
			}
		}

		if !current_volumes.is_empty() {
			// Preventing removing number of volume
			return Err(Error::empty())
				.status(400)
				.body(error!(CANNOT_REMOVE_VOLUME).to_string());
		}
	}

	if let Some(environment_variables) = environment_variables {
		log::trace!(
			"request_id: {} - Updating deployment environment variables in the database",
			request_id
		);
		db::remove_all_environment_variables_for_deployment(
			connection,
			deployment_id,
		)
		.await?;
		for (key, value) in environment_variables {
			match value {
				EnvironmentVariableValue::String(value) => {
					db::add_environment_variable_for_deployment(
						connection,
						deployment_id,
						key,
						Some(value),
						None,
					)
				}
				EnvironmentVariableValue::Secret {
					from_secret: secret_id,
				} => db::add_environment_variable_for_deployment(
					connection,
					deployment_id,
					key,
					None,
					Some(secret_id),
				),
			}
			.await?;
		}
	}
	log::trace!(
		"request_id: {} - Deployment updated in the database",
		request_id
	);

	let (deployment, _, full_image, running_details) =
		get_full_deployment_config(connection, deployment_id, request_id)
			.await?;

	let volumes =
		db::get_all_deployment_volumes(connection, deployment_id).await?;

	let (kubeconfig, deployed_region_id) =
		service::get_kubernetes_config_for_region(
			connection,
			&deployment.region,
		)
		.await?;

	if let Some(new_min_replica) = min_horizontal_scale {
		if new_min_replica != old_min_replicas {
			let is_patr_cluster = service::is_deployed_on_patr_cluster(
				connection,
				&deployment.region,
			)
			.await?;
			for volume in &volumes {
				if is_patr_cluster {
					db::stop_volume_usage_history(
						connection,
						&volume.volume_id,
						&now,
					)
					.await?;
					db::start_volume_usage_history(
						connection,
						workspace_id,
						&volume.volume_id,
						volume.size as u64 * 1000u64 * 1000u64 * 1000u64,
						new_min_replica,
						&now,
					)
					.await?;
				}

				if new_min_replica < old_min_replicas {
					// Delete any excess volumes that are present
					for replica_index in new_min_replica..old_min_replicas {
						service::delete_kubernetes_volume(
							workspace_id,
							deployment_id,
							volume,
							replica_index,
							kubeconfig.clone(),
							request_id,
						)
						.await?;
					}
				}
			}
		}
	}

	match &deployment.status {
		DeploymentStatus::Stopped |
		DeploymentStatus::Deleted |
		DeploymentStatus::Created => {
			// Don't update deployments that are explicitly stopped or deleted
		}
		_ => {
			db::update_deployment_status(
				connection,
				deployment_id,
				&DeploymentStatus::Deploying,
			)
			.await?;

			if service::is_deployed_on_patr_cluster(
				connection,
				&deployment.region,
			)
			.await?
			{
				db::stop_deployment_usage_history(
					connection,
					deployment_id,
					&now,
				)
				.await?;

				db::start_deployment_usage_history(
					connection,
					workspace_id,
					deployment_id,
					&deployment.machine_type,
					running_details.min_horizontal_scale as i32,
					&now,
				)
				.await?;
			}

			for volume in &volumes {
				service::update_kubernetes_volume(
					workspace_id,
					deployment_id,
					volume,
					running_details.min_horizontal_scale,
					kubeconfig.clone(),
					request_id,
				)
				.await?;
			}

			service::update_kubernetes_deployment(
				workspace_id,
				&deployment,
				&full_image,
				None,
				&running_details,
				&volumes,
				kubeconfig,
				&deployed_region_id,
				config,
				request_id,
			)
			.await?;

			service::update_cloudflare_kv_for_deployment(
				deployment_id,
				cloudflare::deployment::Value::Running {
					region_id: deployed_region_id,
					ports: running_details
						.ports
						.iter()
						.map(|(port, _type)| port.value())
						.collect(),
				},
				config,
			)
			.await?;
		}
	}

	Ok(())
}

pub async fn get_full_deployment_config(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	request_id: &Uuid,
) -> Result<(Deployment, Uuid, String, DeploymentRunningDetails), Error> {
	log::trace!(
		"request_id: {} - Getting the full deployment config for deployment with id: {}",
		request_id,
		deployment_id
	);
	let (
		deployment,
		workspace_id,
		deploy_on_push,
		min_horizontal_scale,
		max_horizontal_scale,
		startup_probe_port,
		startup_probe_path,
		liveness_probe_port,
		liveness_probe_path,
	) = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.and_then(|deployment| {
			Some((
				Deployment {
					id: deployment.id,
					name: deployment.name,
					registry: if deployment.registry == constants::PATR_REGISTRY
					{
						DeploymentRegistry::PatrRegistry {
							registry: PatrRegistry,
							repository_id: deployment.repository_id?,
						}
					} else {
						DeploymentRegistry::ExternalRegistry {
							registry: deployment.registry,
							image_name: deployment.image_name?,
						}
					},
					image_tag: deployment.image_tag,
					status: deployment.status,
					region: deployment.region,
					machine_type: deployment.machine_type,
					current_live_digest: deployment.current_live_digest,
				},
				deployment.workspace_id,
				deployment.deploy_on_push,
				deployment.min_horizontal_scale as u16,
				deployment.max_horizontal_scale as u16,
				deployment.startup_probe_port,
				deployment.startup_probe_path,
				deployment.liveness_probe_port,
				deployment.liveness_probe_path,
			))
		})
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let full_image = match &deployment.registry {
		DeploymentRegistry::PatrRegistry {
			registry: _,
			repository_id,
		} => {
			let repository =
				db::get_docker_repository_by_id(connection, repository_id)
					.await?
					.status(404)
					.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

			format!(
				"{}/{}/{}",
				constants::PATR_REGISTRY,
				repository.workspace_id,
				repository.name
			)
		}
		DeploymentRegistry::ExternalRegistry {
			registry,
			image_name,
		} => {
			format!("{}/{}", registry, image_name)
		}
	};

	let ports = db::get_exposed_ports_for_deployment(connection, deployment_id)
		.await?
		.into_iter()
		.map(|(port, port_type)| (StringifiedU16::new(port), port_type))
		.collect();

	let environment_variables =
		db::get_environment_variables_for_deployment(connection, deployment_id)
			.await?
			.into_iter()
			.filter_map(|env| match (env.value, env.secret_id) {
				(Some(value), None) => {
					Some((env.name, EnvironmentVariableValue::String(value)))
				}
				(None, Some(secret_id)) => Some((
					env.name,
					EnvironmentVariableValue::Secret {
						from_secret: secret_id,
					},
				)),
				_ => None,
			})
			.collect();

	let config_mounts =
		db::get_all_deployment_config_mounts(connection, deployment_id)
			.await?
			.into_iter()
			.map(|mount| (mount.path, mount.file.into()))
			.collect();

	let volumes = db::get_all_deployment_volumes(connection, deployment_id)
		.await?
		.into_iter()
		.map(|volume| {
			(
				volume.name,
				DeploymentVolume {
					path: volume.path,
					size: volume.size as u16,
				},
			)
		})
		.collect();

	log::trace!("request_id: {} - Full deployment config for deployment with id: {} successfully retreived", request_id, deployment_id);

	Ok((
		deployment,
		workspace_id,
		full_image,
		DeploymentRunningDetails {
			deploy_on_push,
			min_horizontal_scale,
			max_horizontal_scale,
			ports,
			environment_variables,
			startup_probe: startup_probe_port
				.map(|port| port as u16)
				.zip(startup_probe_path)
				.map(|(port, path)| DeploymentProbe { path, port }),
			liveness_probe: liveness_probe_port
				.map(|port| port as u16)
				.zip(liveness_probe_path)
				.map(|(port, path)| DeploymentProbe { path, port }),
			config_mounts,
			volumes,
		},
	))
}

pub async fn get_deployment_metrics(
	deployment_id: &Uuid,
	config: &Settings,
	start_time: &DateTime<Utc>,
	end_time: &DateTime<Utc>,
	step: &str,
	request_id: &Uuid,
) -> Result<Vec<DeploymentMetrics>, Error> {
	log::trace!(
		"request_id: {} - Getting deployment metrics for deployment with id: {}",
		request_id,
		deployment_id
	);

	// TODO: make this as a hashmap
	let mut metric_response = Vec::<DeploymentMetrics>::new();
	let client = Client::new();

	let (
		cpu_usage_response,
		memory_usage_response,
		network_usage_tx_response,
		network_usage_rx_response,
	): (
		Result<_, reqwest::Error>,
		Result<_, reqwest::Error>,
		Result<_, reqwest::Error>,
		Result<_, reqwest::Error>,
	) = tokio::join!(
		async {
			log::trace!("request_id: {} - Getting cpu metrics", request_id);
			client
				.post(format!(
					concat!(
						"https://{}/prometheus/api/v1/query_range?query=",
						"sum(rate(container_cpu_usage_seconds_total",
						"{{pod=~\"deployment-{}-(.*)\"}}[{step}])) by (pod)",
						"&start={}&end={}&step={step}"
					),
					config.mimir.host,
					deployment_id,
					start_time.timestamp(),
					end_time.timestamp(),
					step = step
				))
				.basic_auth(
					&config.mimir.username,
					Some(&config.mimir.password),
				)
				.send()
				.await?
				.json::<PrometheusResponse>()
				.await
		},
		async {
			log::trace!(
				"request_id: {} - Getting memory usage metrics",
				request_id
			);
			client
				.post(format!(
					concat!(
						"https://{}/prometheus/api/v1/query_range?query=",
						"sum(rate(container_memory_usage_bytes",
						"{{pod=~\"deployment-{}-(.*)\"}}[{step}])) by (pod)",
						"&start={}&end={}&step={step}"
					),
					config.mimir.host,
					deployment_id,
					start_time.timestamp(),
					end_time.timestamp(),
					step = step
				))
				.basic_auth(
					&config.mimir.username,
					Some(&config.mimir.password),
				)
				.send()
				.await?
				.json::<PrometheusResponse>()
				.await
		},
		async {
			log::trace!(
				"request_id: {} - Getting network usage transmit metrics",
				request_id
			);
			client
				.post(format!(
					concat!(
						"https://{}/prometheus/api/v1/query_range?query=",
						"sum(rate(container_network_transmit_bytes_total",
						"{{pod=~\"deployment-{}-(.*)\"}}[{step}])) by (pod)",
						"&start={}&end={}&step={step}"
					),
					config.mimir.host,
					deployment_id,
					start_time.timestamp(),
					end_time.timestamp(),
					step = step
				))
				.basic_auth(
					&config.mimir.username,
					Some(&config.mimir.password),
				)
				.send()
				.await?
				.json::<PrometheusResponse>()
				.await
		},
		async {
			log::trace!(
				"request_id: {} - Getting network usage recieve metrics",
				request_id
			);
			client
				.post(format!(
					concat!(
						"https://{}/prometheus/api/v1/query_range?query=",
						"sum(rate(container_network_receive_bytes_total",
						"{{pod=~\"deployment-{}-(.*)\"}}[{step}])) by (pod)",
						"&start={}&end={}&step={step}"
					),
					config.mimir.host,
					deployment_id,
					start_time.timestamp(),
					end_time.timestamp(),
					step = step
				))
				.basic_auth(
					&config.mimir.username,
					Some(&config.mimir.password),
				)
				.send()
				.await?
				.json::<PrometheusResponse>()
				.await
		}
	);

	// TODO: this part handles error, however it is not handled properly
	// there is a possibility that the error that we get from prometheus is not
	// a reqwest error but a prometheus error, in that case we should handle it
	// properly and show it to the user
	let (
		cpu_usage_response,
		memory_usage_response,
		network_usage_tx_response,
		network_usage_rx_response,
	) = (
		cpu_usage_response?,
		memory_usage_response?,
		network_usage_tx_response?,
		network_usage_rx_response?,
	);

	log::trace!(
		"request_id: {} - mapping cpu usage metrics on to response struct",
		request_id
	);
	cpu_usage_response
		.data
		.result
		.into_iter()
		.for_each(|prom_metric| {
			let pod_item = if let Some(item) = metric_response
				.iter_mut()
				.find(|item| item.pod_name == prom_metric.metric.pod)
			{
				item
			} else {
				let new_item = DeploymentMetrics {
					pod_name: prom_metric.metric.pod,
					metrics: vec![],
				};

				let metric_response_size = metric_response.len();

				metric_response.push(new_item);
				if let Some(item) =
					metric_response.get_mut(metric_response_size)
				{
					item
				} else {
					return;
				}
			};

			prom_metric.values.into_iter().for_each(|value| {
				let metric_item = if let Some(item) = pod_item
					.metrics
					.iter_mut()
					.find(|item| item.timestamp == value.timestamp)
				{
					item
				} else {
					let new_item = Metric {
						timestamp: value.timestamp,
						cpu_usage: "0".to_string(),
						memory_usage: "0".to_string(),
						network_usage_tx: "0".to_string(),
						network_usage_rx: "0".to_string(),
					};

					let pod_item_size = pod_item.metrics.len();

					pod_item.metrics.push(new_item);
					if let Some(item) = pod_item.metrics.get_mut(pod_item_size)
					{
						item
					} else {
						return;
					}
				};
				metric_item.cpu_usage = value.value;
			});
		});

	log::trace!(
		"request_id: {} - mapping memory usage metrics on to response struct",
		request_id
	);
	memory_usage_response
		.data
		.result
		.into_iter()
		.for_each(|prom_metric| {
			let pod_item = if let Some(item) = metric_response
				.iter_mut()
				.find(|item| item.pod_name == prom_metric.metric.pod)
			{
				item
			} else {
				let new_item = DeploymentMetrics {
					pod_name: prom_metric.metric.pod,
					metrics: vec![],
				};

				let metric_response_size = metric_response.len();

				metric_response.push(new_item);
				if let Some(item) =
					metric_response.get_mut(metric_response_size)
				{
					item
				} else {
					return;
				}
			};

			prom_metric.values.into_iter().for_each(|value| {
				let metric_item = if let Some(item) = pod_item
					.metrics
					.iter_mut()
					.find(|item| item.timestamp == value.timestamp)
				{
					item
				} else {
					return;
				};
				metric_item.memory_usage = value.value;
			});
		});

	log::trace!("request_id: {} - mapping network usage transmit metrics on to response struct", request_id);
	network_usage_tx_response
		.data
		.result
		.into_iter()
		.for_each(|prom_metric| {
			let pod_item = if let Some(item) = metric_response
				.iter_mut()
				.find(|item| item.pod_name == prom_metric.metric.pod)
			{
				item
			} else {
				let new_item = DeploymentMetrics {
					pod_name: prom_metric.metric.pod,
					metrics: vec![],
				};

				let metric_response_size = metric_response.len();

				metric_response.push(new_item);
				if let Some(item) =
					metric_response.get_mut(metric_response_size)
				{
					item
				} else {
					return;
				}
			};

			prom_metric.values.into_iter().for_each(|value| {
				let metric_item = if let Some(item) = pod_item
					.metrics
					.iter_mut()
					.find(|item| item.timestamp == value.timestamp)
				{
					item
				} else {
					return;
				};
				metric_item.network_usage_tx = value.value;
			});
		});

	log::trace!("request_id: {} - mapping network usage receive metrics on to response struct", request_id);
	network_usage_rx_response
		.data
		.result
		.into_iter()
		.for_each(|prom_metric| {
			let pod_item = if let Some(item) = metric_response
				.iter_mut()
				.find(|item| item.pod_name == prom_metric.metric.pod)
			{
				item
			} else {
				let new_item = DeploymentMetrics {
					pod_name: prom_metric.metric.pod,
					metrics: vec![],
				};

				let metric_response_size = metric_response.len();

				metric_response.push(new_item);
				if let Some(item) =
					metric_response.get_mut(metric_response_size)
				{
					item
				} else {
					return;
				}
			};

			prom_metric.values.into_iter().for_each(|value| {
				let metric_item = if let Some(item) = pod_item
					.metrics
					.iter_mut()
					.find(|item| item.timestamp == value.timestamp)
				{
					item
				} else {
					return;
				};
				metric_item.network_usage_rx = value.value;
			});
		});

	Ok(metric_response)
}

async fn get_container_logs(
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	start_time: &DateTime<Utc>,
	end_time: &DateTime<Utc>,
	limit: u32,
	config: &Settings,
	request_id: &Uuid,
) -> Result<Vec<DeploymentLogs>, Error> {
	log::trace!(
		"request_id: {} - Getting container logs for deployment with id: {}",
		request_id,
		deployment_id
	);
	let client = Client::new();
	let logs = client
		.get(format!(
			concat!(
				"https://{}/loki/api/v1/query_range?direction=BACKWARD&",
				"query={{container=\"deployment-{}\",namespace=\"{}\"}}",
				"&start={}&end={}&limit={}"
			),
			config.loki.host,
			deployment_id,
			workspace_id,
			start_time.timestamp_nanos(),
			end_time.timestamp_nanos(),
			limit
		))
		.basic_auth(&config.loki.username, Some(&config.loki.password))
		.send()
		.await?
		.json::<Logs>()
		.await?
		.data
		.result;

	log::trace!(
		"request_id: {} - successful retrieved container logs for deployment with id: {}",
		request_id,
		deployment_id
	);

	let mut logs = logs
		.into_iter()
		.flat_map(|loki_logs| loki_logs.values)
		.filter_map(|log| Some((log[0].parse::<u64>().ok()?, log[1].clone())))
		.map(|(timestamp, log)| DeploymentLogs {
			timestamp: TzDateTime(Utc.timestamp_nanos(timestamp as i64)),
			logs: log,
		})
		.collect::<Vec<_>>();

	logs.sort_by(|a, b| a.timestamp.0.cmp(&b.timestamp.0));

	Ok(logs)
}

pub async fn get_deployment_build_logs(
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	start_time: &DateTime<Utc>,
	end_time: &DateTime<Utc>,
	config: &Settings,
	request_id: &Uuid,
) -> Result<Vec<Event>, Error> {
	log::trace!(
		"request_id: {} - Getting build logs for deployment with id: {}",
		request_id,
		deployment_id
	);
	let client = Client::new();
	let logs = client
		.get(format!(
			concat!(
				"https://{}/loki/api/v1/query_range?direction=BACKWARD&",
				"query={{app=\"eventrouter\",namespace=\"{}\"}}",
				"&start={}&end={}"
			),
			config.loki.host,
			workspace_id,
			start_time.timestamp_nanos(),
			end_time.timestamp_nanos()
		))
		.basic_auth(&config.loki.username, Some(&config.loki.password))
		.send()
		.await?
		.json::<Logs>()
		.await?
		.data
		.result;

	// TODO: you will get only one element in result array. From that result
	// array get the element and parse that json and from that json filter the
	// logs.

	let result = logs.into_iter().next().status(500)?;

	let mut combined_build_logs = Vec::new();

	for value in result.values {
		let kube_event = value.into_iter().next().status(500)?;

		let kube_event: Event = serde_json::from_str(kube_event.as_str())?;

		let namespace =
			kube_event.clone().metadata.namespace.status(500)?.clone();
		let deployment_name =
			kube_event.clone().metadata.name.status(500)?.clone();

		if namespace == workspace_id.to_string() &&
			deployment_name == format!("deployment-{}", deployment_id)
		{
			combined_build_logs.push(kube_event);
		}
	}

	Ok(combined_build_logs)
}

async fn check_deployment_creation_limit(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	is_byoc_region: bool,
	machine_type: &Uuid,
	min_horizontal_scale: &u16,
	max_horizontal_scale: &u16,
	volumes: &BTreeMap<String, DeploymentVolume>,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {request_id} - Checking whether new deployment creation is limited");

	if is_byoc_region {
		// if byoc, then don't need to check free/paid/total limits
		// as this deloyment is going to be deployed on their cluster
		return Ok(());
	}

	let current_deployment_count =
		db::get_deployments_for_workspace(connection, workspace_id)
			.await?
			.len();

	let volume_size = volumes
		.iter()
		.map(|(_, volume)| volume.size as u32)
		.sum::<u32>();

	let card_added =
		db::get_default_payment_method_for_workspace(connection, workspace_id)
			.await?
			.is_some();

	if !card_added {
		// check whether free limit is exceeded
		if current_deployment_count >= free_limits::DEPLOYMENT_COUNT {
			log::info!("request_id: {request_id} - Free deployment limit reached and card is not added");
			return Error::as_result()
				.status(400)
				.body(error!(CARDLESS_FREE_LIMIT_EXCEEDED).to_string())?;
		}

		// only basic machine type is allowed under free plan
		let machine_type_to_be_deployed = MACHINE_TYPES
			.get()
			.and_then(|machines| machines.get(machine_type))
			.status(500)?;

		if machine_type_to_be_deployed != &(1, 2) {
			log::info!("request_id: {request_id} - Only basic machine type is allowed under free plan");
			return Error::as_result().status(400).body(
				error!(CARDLESS_DEPLOYMENT_MACHINE_TYPE_LIMIT).to_string(),
			)?;
		}

		if *max_horizontal_scale > 1 || *min_horizontal_scale > 1 {
			log::info!("request_id: {request_id} - Only one replica allowed under free plan without card");
			return Error::as_result()
				.status(400)
				.body(error!(REPLICA_LIMIT_EXCEEDED).to_string())?;
		}

		let volume_size_in_byte = volume_size as usize * 1024 * 1024 * 1024;
		if volume_size_in_byte > free_limits::VOLUME_STORAGE_IN_BYTE {
			return Error::as_result()
				.status(400)
				.body(error!(CARDLESS_VOLUME_LIMIT_EXCEEDED).to_string())?;
		}
	}

	// check whether max deployment limit is exceeded
	let workspace = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	if current_deployment_count >= workspace.database_limit as usize {
		log::info!(
			"request_id: {request_id} - Max deployment limit for workspace reached"
		);
		return Error::as_result()
			.status(400)
			.body(error!(DEPLOYMENT_LIMIT_EXCEEDED).to_string())?;
	}

	if volume_size > workspace.volume_storage_limit as u32 {
		return Error::as_result()
			.status(400)
			.body(error!(VOLUME_LIMIT_EXCEEDED).to_string())?;
	}

	// check whether total resource limit is exceeded
	if super::resource_limit_crossed(connection, workspace_id, request_id)
		.await?
	{
		log::info!("request_id: {request_id} - Total resource limit exceeded");
		return Error::as_result()
			.status(400)
			.body(error!(RESOURCE_LIMIT_EXCEEDED).to_string())?;
	}

	Ok(())
}

pub async fn start_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	deployment: &Deployment,
	deployment_running_details: &DeploymentRunningDetails,
	user_id: &Uuid,
	login_id: &Uuid,
	ip_address: &str,
	metadata: &DeploymentMetadata,
	current_time: &DateTime<Utc>,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	// If deploy_on_create is true, then tell the consumer to create a
	// deployment
	let (image_name, digest) =
		service::get_image_name_and_digest_for_deployment_image(
			connection,
			&deployment.registry,
			&deployment.image_tag,
			config,
			request_id,
		)
		.await?;

	db::update_deployment_status(
		connection,
		deployment_id,
		&DeploymentStatus::Deploying,
	)
	.await?;

	let volumes =
		db::get_all_deployment_volumes(connection, deployment_id).await?;

	if service::is_deployed_on_patr_cluster(connection, &deployment.region)
		.await?
	{
		for volume in &volumes {
			log::trace!(
				"request_id: {} starting volume usage history",
				request_id
			);

			// TODO - Figure out a better solution to this. If possible try
			// doing it on db layer.The aim is to handle the case when user has
			// created the deployment but deploy_on_create is false.
			// start_deployment is called from create, update and start
			// deployment routes in which case we have to handle the case when
			// user is starting is starting the deployment for the first time
			// where volume_usage table is empty also considering that volume
			// usage only stops when deployment is permanently deleted
			if db::get_volume_payment_history_by_volume_id(
				connection,
				&volume.volume_id,
			)
			.await?
			.is_none()
			{
				db::start_volume_usage_history(
					connection,
					workspace_id,
					&volume.volume_id,
					volume.size as u64 * 1000u64 * 1000u64 * 1000u64,
					deployment_running_details.min_horizontal_scale,
					current_time,
				)
				.await?;
			}
		}

		db::start_deployment_usage_history(
			connection,
			workspace_id,
			deployment_id,
			&deployment.machine_type,
			deployment_running_details.min_horizontal_scale as i32,
			current_time,
		)
		.await?;
	}

	let audit_log_id =
		db::generate_new_workspace_audit_log_id(connection).await?;

	db::create_workspace_audit_log(
		connection,
		&audit_log_id,
		workspace_id,
		ip_address,
		&Utc::now(),
		Some(user_id),
		Some(login_id),
		&deployment.id,
		rbac::PERMISSIONS
			.get()
			.unwrap()
			.get(permissions::workspace::infrastructure::deployment::EDIT)
			.unwrap(),
		request_id,
		&serde_json::to_value(metadata)?,
		false,
		true,
	)
	.await?;

	let workspace = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)?;

	if workspace.is_spam {
		return Err(Error::empty()
			.status(401)
			.body(error!(UNVERIFIED_WORKSPACE).to_string()));
	}

	let (kubeconfig, deployed_region_id) =
		service::get_kubernetes_config_for_region(
			connection,
			&deployment.region,
		)
		.await?;

	service::update_kubernetes_deployment(
		workspace_id,
		deployment,
		&image_name,
		digest.as_deref(),
		deployment_running_details,
		&volumes,
		kubeconfig,
		&deployed_region_id,
		config,
		request_id,
	)
	.await?;

	service::update_cloudflare_kv_for_deployment(
		deployment_id,
		cloudflare::deployment::Value::Running {
			region_id: deployed_region_id,
			ports: deployment_running_details
				.ports
				.iter()
				.map(|(port, _type)| port.value())
				.collect(),
		},
		config,
	)
	.await?;

	Ok(())
}

pub async fn update_deployment_image(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	name: &str,
	registry: &DeploymentRegistry,
	digest: &str,
	image_tag: &str,
	image_name: &str,
	region: &Uuid,
	machine_type: &Uuid,
	deployment_running_details: &DeploymentRunningDetails,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let audit_log_id =
		db::generate_new_workspace_audit_log_id(connection).await?;

	db::create_workspace_audit_log(
		connection,
		&audit_log_id,
		workspace_id,
		"0.0.0.0",
		&Utc::now(),
		None,
		None,
		deployment_id,
		rbac::PERMISSIONS
			.get()
			.unwrap()
			.get(permissions::workspace::infrastructure::deployment::EDIT)
			.unwrap(),
		request_id,
		&serde_json::to_value(DeploymentMetadata::Start {})?,
		true,
		true,
	)
	.await?;

	let workspace = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)?;

	let volumes =
		db::get_all_deployment_volumes(connection, deployment_id).await?;

	if workspace.is_spam {
		db::update_deployment_status(
			connection,
			deployment_id,
			&DeploymentStatus::Running,
		)
		.await?;

		Ok(())
	} else {
		let (kubeconfig, deployed_region_id) =
			service::get_kubernetes_config_for_region(connection, region)
				.await?;

		service::update_kubernetes_deployment(
			workspace_id,
			&Deployment {
				id: deployment_id.clone(),
				name: name.to_string(),
				registry: registry.clone(),
				image_tag: image_tag.to_string(),
				status: DeploymentStatus::Pushed,
				region: region.clone(),
				machine_type: machine_type.clone(),
				current_live_digest: Some(digest.to_string()),
			},
			image_name,
			Some(digest),
			deployment_running_details,
			&volumes,
			kubeconfig,
			&deployed_region_id,
			config,
			request_id,
		)
		.await?;

		Ok(())
	}
}

pub async fn stop_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	region_id: &Uuid,
	user_id: &Uuid,
	login_id: &Uuid,
	ip_address: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	// TODO: implement logic for handling domains of the stopped deployment
	log::trace!(
		"request_id: {} - Updating deployment status as stopped",
		request_id
	);
	db::update_deployment_status(
		connection,
		deployment_id,
		&DeploymentStatus::Stopped,
	)
	.await?;

	if service::is_deployed_on_patr_cluster(connection, region_id).await? {
		db::stop_deployment_usage_history(
			connection,
			deployment_id,
			&Utc::now(),
		)
		.await?;
	}

	let audit_log_id =
		db::generate_new_workspace_audit_log_id(connection).await?;

	db::create_workspace_audit_log(
		connection,
		&audit_log_id,
		workspace_id,
		ip_address,
		&Utc::now(),
		Some(user_id),
		Some(login_id),
		deployment_id,
		rbac::PERMISSIONS
			.get()
			.unwrap()
			.get(permissions::workspace::infrastructure::deployment::EDIT)
			.unwrap(),
		request_id,
		&serde_json::to_value(DeploymentMetadata::Stop {})?,
		false,
		true,
	)
	.await?;

	let (kubeconfig, _) =
		service::get_kubernetes_config_for_region(connection, region_id)
			.await?;

	service::delete_kubernetes_deployment(
		workspace_id,
		deployment_id,
		kubeconfig,
		request_id,
	)
	.await?;

	service::update_cloudflare_kv_for_deployment(
		deployment_id,
		cloudflare::deployment::Value::Stopped,
		config,
	)
	.await?;

	Ok(())
}

pub async fn delete_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	region_id: &Uuid,
	user_id: Option<&Uuid>,
	login_id: Option<&Uuid>,
	ip_address: &str,
	patr_action: bool,
	delete_k8s_resource: bool,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Updating the deployment deletion time in the database",
		request_id
	);

	let now = Utc::now();

	let min_replicas = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(500)?
		.min_horizontal_scale as u16;

	db::delete_deployment(connection, deployment_id, &Utc::now()).await?;

	let audit_log_id =
		db::generate_new_workspace_audit_log_id(connection).await?;
	db::create_workspace_audit_log(
		connection,
		&audit_log_id,
		workspace_id,
		ip_address,
		&now,
		user_id,
		login_id,
		deployment_id,
		rbac::PERMISSIONS
			.get()
			.unwrap()
			.get(permissions::workspace::infrastructure::deployment::EDIT)
			.unwrap(),
		request_id,
		&serde_json::to_value(DeploymentMetadata::Delete {})?,
		patr_action,
		true,
	)
	.await?;

	let workspace = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)?;

	if delete_k8s_resource && !workspace.is_spam {
		let (kubeconfig, _) =
			service::get_kubernetes_config_for_region(connection, region_id)
				.await?;

		service::delete_kubernetes_deployment(
			workspace_id,
			deployment_id,
			kubeconfig.clone(),
			request_id,
		)
		.await?;

		let volumes =
			db::get_all_deployment_volumes(connection, deployment_id).await?;

		for volume in volumes {
			db::delete_volume(connection, &volume.volume_id, &now).await?;

			for replica_index in 0..min_replicas {
				service::delete_kubernetes_volume(
					workspace_id,
					deployment_id,
					&volume,
					replica_index,
					kubeconfig.clone(),
					request_id,
				)
				.await?;
			}
		}
	}

	service::update_cloudflare_kv_for_deployment(
		deployment_id,
		cloudflare::deployment::Value::Deleted,
		config,
	)
	.await?;

	Ok(())
}
