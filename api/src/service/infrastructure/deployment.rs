use std::{collections::BTreeMap, str};

use api_models::{
	models::workspace::infrastructure::deployment::{
		Deployment,
		DeploymentMetrics,
		DeploymentRegistry,
		DeploymentRunningDetails,
		EnvironmentVariableValue,
		ExposedPortType,
		Metric,
		PatrRegistry,
	},
	utils::{constants, StringifiedU16, Uuid},
};
use eve_rs::AsError;
use reqwest::Client;

use crate::{
	db,
	error,
	models::{
		deployment::{PrometheusResponse, Subscription},
		rbac,
	},
	service::infrastructure::kubernetes,
	utils::{get_current_time_millis, settings::Settings, validator, Error},
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
	config: &Settings,
	request_id: &Uuid,
) -> Result<Uuid, Error> {
	// As of now, only our custom registry is allowed
	// Docker hub will also be allowed in the near future
	log::trace!("request_id: {} - Checking if the deployment's image is in patr registry", request_id);
	if !registry.is_patr_registry() {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	if image_tag.is_empty() {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	// validate deployment name
	log::trace!("request_id: {} - Validating deployment name", request_id);
	if !validator::is_deployment_name_valid(name) {
		return Err(Error::empty()
			.status(200)
			.body(error!(INVALID_DEPLOYMENT_NAME).to_string()));
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

	db::create_resource(
		connection,
		&deployment_id,
		&format!("Deployment: {}", name),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::DEPLOYMENT)
			.unwrap(),
		workspace_id,
		get_current_time_millis(),
	)
	.await?;
	log::trace!("request_id: {} - Created resource", request_id);

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

	for (key, value) in &deployment_running_details.environment_variables {
		log::trace!(
			"request_id: {} - Adding environment variable entry to database",
			request_id
		);
		db::add_environment_variable_for_deployment(
			connection,
			&deployment_id,
			key,
			if let EnvironmentVariableValue::String(value) = value {
				value
			} else {
				return Err(Error::empty()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string()));
			},
		)
		.await?;
	}

	start_subscription(
		connection,
		&deployment_id,
		machine_type.as_str(),
		deployment_running_details.min_horizontal_scale,
		config,
		request_id,
	)
	.await?;

	Ok(deployment_id)
}

pub async fn get_deployment_container_logs(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<String, Error> {
	log::trace!(
		"Getting deployment logs for deployment_id: {} with request_id: {}",
		deployment_id,
		request_id
	);

	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let logs = kubernetes::get_container_logs(
		&deployment.workspace_id,
		deployment_id,
		config,
		request_id,
	)
	.await?;
	log::trace!("request_id: {} - Logs retreived successfully", request_id);

	Ok(logs)
}

pub async fn update_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	name: Option<&str>,
	region: Option<&Uuid>,
	machine_type: Option<&Uuid>,
	deploy_on_push: Option<bool>,
	min_horizontal_scale: Option<u16>,
	max_horizontal_scale: Option<u16>,
	ports: Option<&BTreeMap<u16, ExposedPortType>>,
	environment_variables: Option<&BTreeMap<String, EnvironmentVariableValue>>,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Updating deployment with id: {}",
		request_id,
		deployment_id
	);
	db::update_deployment_details(
		connection,
		deployment_id,
		name,
		region,
		machine_type,
		deploy_on_push,
		min_horizontal_scale,
		max_horizontal_scale,
	)
	.await?;

	if let Some(ports) = ports {
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
			db::add_environment_variable_for_deployment(
				connection,
				deployment_id,
				key,
				if let EnvironmentVariableValue::String(value) = value {
					value
				} else {
					return Err(Error::empty()
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string()));
				},
			)
			.await?;
		}
	}
	log::trace!(
		"request_id: {} - Deployment updated in the database",
		request_id
	);

	update_subscription(connection, deployment_id, config, request_id).await?;

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
				},
				deployment.workspace_id,
				deployment.deploy_on_push,
				deployment.min_horizontal_scale as u16,
				deployment.max_horizontal_scale as u16,
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
			let workspace =
				db::get_workspace_info(connection, &repository.workspace_id)
					.await?
					.status(500)?;
			format!(
				"{}/{}/{}",
				constants::PATR_REGISTRY,
				workspace.name,
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
			.map(|(key, value)| (key, EnvironmentVariableValue::String(value)))
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
		},
	))
}

pub async fn get_deployment_metrics(
	deployment_id: &Uuid,
	config: &Settings,
	start_time: u64,
	end_time: u64,
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
			client.post(format!("https://{}/api/v1/query_range?query=sum(rate(container_cpu_usage_seconds_total{{pod=~\"deployment-{}-(.*)\"}}[{step}])) by (pod)&start={}&end={}&step={step}", config.prometheus.host, deployment_id, start_time, end_time))
				.basic_auth(&config.prometheus.username, Some(&config.prometheus.password))
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
			client.post(format!("https://{}/api/v1/query_range?query=sum(rate(container_memory_usage_bytes{{pod=~\"deployment-{}-(.*)\"}}[{step}])) by (pod)&start={}&end={}&step={step}", config.prometheus.host, deployment_id, start_time, end_time))
				.basic_auth(&config.prometheus.username, Some(&config.prometheus.password))
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
			client.post(format!("https://{}/api/v1/query_range?query=sum(rate(container_network_transmit_bytes_total{{pod=~\"deployment-{}-(.*)\"}}[{step}])) by (pod)&start={}&end={}&step={step}", config.prometheus.host, deployment_id, start_time, end_time))
				.basic_auth(&config.prometheus.username, Some(&config.prometheus.password))
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
			client.post(format!("https://{}/api/v1/query_range?query=sum(rate(container_network_receive_bytes_total{{pod=~\"deployment-{}-(.*)\"}}[{step}])) by (pod)&start={}&end={}&step={step}", config.prometheus.host, deployment_id, start_time, end_time))
				.basic_auth(&config.prometheus.username, Some(&config.prometheus.password))
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

pub async fn cancel_subscription(
	deployment_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Stopping subscription for deployment with id: {}",
		request_id,
		deployment_id
	);
	let client = Client::new();

	let password: Option<String> = None;

	let status = client
		.post(format!(
			"{}/subscriptions/{}/cancel_for_items",
			config.chargebee.url, deployment_id
		))
		.basic_auth(&config.chargebee.api_key, password)
		.query(&[
			("end_of_term", "false"),
			("credit_option_for_current_term_charges", "prorate"),
		])
		.send()
		.await?
		.status();

	if status.is_client_error() || status.is_server_error() {
		log::error!(
			"request_id: {} - Error with the deployment: {}",
			request_id,
			status,
		);
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string());
	}

	Ok(())
}

async fn start_subscription(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	item_price_id: &str,
	quantity: u16,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Starting subscription for deployment with id: {}",
		request_id,
		deployment_id
	);
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;
	let client = Client::new();
	let password: Option<String> = None;
	let status = client
		.post(format!(
			"{}/customers/{}/subscription_for_items",
			config.chargebee.url, deployment.workspace_id
		))
		.basic_auth(&config.chargebee.api_key, password)
		.query(&Subscription {
			id: deployment_id.to_string(),
			item_price_id: format!("{}-USD-Monthly", item_price_id),
			quantity,
		})
		.send()
		.await?
		.status();
	if status.is_client_error() || status.is_server_error() {
		log::error!(
			"request_id: {} - Error with the deployment: {}",
			request_id,
			status,
		);
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string());
	}
	Ok(())
}

async fn update_subscription(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Updating subscription for deployment with id: {}",
		request_id,
		deployment_id
	);

	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let client = Client::new();

	let password: Option<String> = None;

	let status = client
		.post(format!(
			"{}/subscriptions/{}/update_for_items",
			config.chargebee.url, deployment.id,
		))
		.basic_auth(&config.chargebee.api_key, password)
		.query(&[
			(
				"subscription_items[item_price_id][0]",
				&format!("{}-USD-Monthly", deployment.machine_type),
			),
			(
				"subscription_items[quantity][0]",
				&format!("{}", deployment.min_horizontal_scale),
			),
		])
		.send()
		.await?
		.status();

	if status.is_client_error() || status.is_server_error() {
		log::error!(
			"request_id: {} - Error with the deployment: {}",
			request_id,
			status,
		);
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string());
	}

	Ok(())
}
