use std::collections::BTreeMap;

use api_models::{
	models::prelude::*,
	utils::{constants, DateTime, Uuid},
};
use axum::{extract::State, Extension, Router};
use chrono::{Duration, Utc};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;

use crate::{
	app::App,
	db::{self, ManagedUrlType as DbManagedUrlType},
	models::{
		cloudflare::deployment,
		rbac::{self, permissions},
		DeploymentMetadata,
		ResourceType,
		UserAuthenticationData,
	},
	prelude::*,
	routes,
	service,
	utils::Error,
};

pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::deployment::LIST,
				|ListDeploymentsPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			list_deployments,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::deployment::INFO,
				|ListDeploymentHistoryPath {
				     workspace_id,
				     deployment_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &deployment_id)
						.await?
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			list_deployment_history,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::deployment::CREATE,
				|CreateDeploymentPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			create_deployment,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::deployment::INFO,
				|GetDeploymentInfoPath {
				     workspace_id,
				     deployment_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &deployment_id)
						.await?
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			get_deployment_info,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::deployment::EDIT,
				|StartDeploymentPath {
				     workspace_id,
				     deployment_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &deployment_id)
						.await?
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			start_deployment,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::deployment::EDIT,
				|StopDeploymentPath {
				     workspace_id,
				     deployment_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &deployment_id)
						.await?
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			stop_deployment,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::deployment::EDIT,
				|RevertDeploymentPath {
				     workspace_id,
				     deployment_id,
				     digest,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &deployment_id)
						.await?
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			revert_deployment,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::deployment::INFO,
				|GetDeploymentBuildLogsPath {
				     workspace_id,
				     deployment_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &deployment_id)
						.await?
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			get_logs,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::deployment::DELETE,
				|DeleteDeploymentPath {
				     workspace_id,
				     deployment_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &deployment_id)
						.await?
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			delete_deployment,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::deployment::EDIT,
				|UpdateDeploymentPath {
				     workspace_id,
				     deployment_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &deployment_id)
						.await?
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			update_deployment,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::managed_url::LIST,
				|ListLinkedURLsPath {
				     workspace_id,
				     deployment_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &deployment_id)
						.await?
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			list_linked_urls,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::deployment::INFO,
				|GetDeploymentMetricsPath {
				     workspace_id,
				     deployment_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &deployment_id)
						.await?
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			get_deployment_metrics,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::deployment::LIST,
				|GetDeploymentBuildLogsPath {
				     workspace_id,
				     deployment_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &deployment_id)
						.await?
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			get_build_logs,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::infrastructure::deployment::INFO,
				|GetDeploymentEventsPath {
				     workspace_id,
				     deployment_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &deployment_id)
						.await?
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			get_build_events,
		)
}

async fn list_deployments(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: ListDeploymentsPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<ListDeploymentsRequest>,
) -> Result<ListDeploymentsResponse, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Listing deployments", request_id);

	log::trace!(
		"request_id: {} - Getting deployments from database",
		request_id
	);
	let deployments =
		db::get_deployments_for_workspace(&mut connection, &workspace_id)
			.await?
			.into_iter()
			.filter_map(|deployment| {
				Some(Deployment {
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
				})
			})
			.collect();
	log::trace!(
		"request_id: {} - Deployments successfully retreived",
		request_id
	);

	Ok(ListDeploymentsResponse { deployments })
}

async fn list_deployment_history(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: ListDeploymentHistoryPath {
			workspace_id,
			deployment_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<ListDeploymentHistoryRequest>,
) -> Result<ListDeploymentHistoryResponse, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Listing deployments", request_id);

	let deployment = db::get_deployment_by_id(&mut connection, &deployment_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	log::trace!(
		"request_id: {} - Getting deployment image digest history from database",
		request_id
	);
	let deploys =
		db::get_all_digest_for_deployment(&mut connection, &deployment_id)
			.await?
			.into_iter()
			.map(|deploy| DeploymentDeployHistory {
				image_digest: deploy.image_digest,
				created: deploy.created.timestamp_millis() as u64,
			})
			.collect();
	log::trace!(
		"request_id: {} - Deployments image history successfully retreived",
		request_id
	);

	Ok(ListDeploymentHistoryResponse { deploys })
}

async fn create_deployment(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: CreateDeploymentPath { workspace_id },
		query: (),
		body:
			CreateDeploymentRequest {
				name,
				registry,
				image_tag,
				region,
				machine_type,
				running_details,
				deploy_on_create,
			},
	}: DecodedRequest<CreateDeploymentRequest>,
) -> Result<CreateDeploymentResponse, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Creating deployment", request_id);

	// TODO - get this fixed in axum
	let ip_address = routes::get_request_ip_address(&context);

	let user_id = token_data.user_id().clone();
	let login_id = token_data.login_id().clone();

	log::trace!(
		"request_id: {} - Creating the deployment in workspace",
		request_id
	);

	let id = service::create_deployment_in_workspace(
		&mut connection,
		&workspace_id,
		&name,
		&registry,
		&image_tag,
		&region,
		&machine_type,
		&running_details,
		&request_id,
	)
	.await?;

	let audit_log_id =
		db::generate_new_workspace_audit_log_id(&mut connection).await?;

	let now = Utc::now();

	let metadata = serde_json::to_value(DeploymentMetadata::Create {
		deployment: Deployment {
			id: id.clone(),
			name: name.to_string(),
			registry: registry.clone(),
			image_tag: image_tag.to_string(),
			status: DeploymentStatus::Created,
			region: region.clone(),
			machine_type: machine_type.clone(),
			current_live_digest: None,
		},
		running_details: running_details.clone(),
	})?;

	db::create_workspace_audit_log(
		&mut connection,
		&audit_log_id,
		&workspace_id,
		&ip_address,
		&now,
		Some(&user_id),
		Some(&login_id),
		&id,
		rbac::PERMISSIONS
			.get()
			.unwrap()
			.get(permissions::workspace::infrastructure::deployment::CREATE)
			.unwrap(),
		&request_id,
		&metadata,
		false,
		true,
	)
	.await?;

	service::update_cloudflare_kv_for_deployment(
		&id,
		deployment::Value::Created,
		&config,
	)
	.await?;

	connection.commit().await?;

	if deploy_on_create {
		let mut is_deployed = false;
		if let DeploymentRegistry::PatrRegistry { repository_id, .. } =
			&registry
		{
			let digest = db::get_latest_digest_for_docker_repository(
				&mut connection,
				repository_id,
			)
			.await?;

			if let Some(digest) = digest {
				db::add_digest_to_deployment_deploy_history(
					&mut connection,
					&id,
					repository_id,
					&digest,
					&now,
				)
				.await?;

				db::update_current_live_digest_for_deployment(
					&mut connection,
					&id,
					&digest,
				)
				.await?;

				if db::get_docker_repository_tag_details(
					&mut connection,
					repository_id,
					&image_tag,
				)
				.await?
				.is_some()
				{
					service::start_deployment(
						&mut connection,
						&workspace_id,
						&id,
						&Deployment {
							id: id.clone(),
							name: name.to_string(),
							registry: registry.clone(),
							image_tag: image_tag.to_string(),
							status: DeploymentStatus::Pushed,
							region: region.clone(),
							machine_type: machine_type.clone(),
							current_live_digest: Some(digest),
						},
						&running_details,
						&user_id,
						&login_id,
						&ip_address,
						&DeploymentMetadata::Start {},
						&now,
						&config,
						&request_id,
					)
					.await?;
					is_deployed = true;
				}
			}
		} else {
			// external registry
			service::start_deployment(
				&mut connection,
				&workspace_id,
				&id,
				&Deployment {
					id: id.clone(),
					name: name.to_string(),
					registry: registry.clone(),
					image_tag: image_tag.to_string(),
					status: DeploymentStatus::Pushed,
					region: region.clone(),
					machine_type: machine_type.clone(),
					current_live_digest: None,
				},
				&running_details,
				&user_id,
				&login_id,
				&ip_address,
				&DeploymentMetadata::Start {},
				&now,
				&config,
				&request_id,
			)
			.await?;
			is_deployed = true;
		}

		if is_deployed {
			connection.commit().await?;

			service::queue_check_and_update_deployment_status(
				&workspace_id,
				&id,
				&config,
				&request_id,
			)
			.await?;
		}
	}

	service::get_internal_metrics(
		&mut connection,
		"A new deployment has been created",
	)
	.await;

	Ok(CreateDeploymentResponse { id })
}

async fn get_deployment_info(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GetDeploymentInfoPath {
			workspace_id,
			deployment_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<GetDeploymentInfoRequest>,
) -> Result<GetDeploymentInfoResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {} - Getting deployment details from the database for deployment: {}",
		request_id,
		deployment_id,
	);
	let (deployment, _, _, running_details) =
		service::get_full_deployment_config(
			&mut connection,
			&deployment_id,
			&request_id,
		)
		.await?;

	Ok(GetDeploymentInfoResponse {
		deployment,
		running_details,
	})
}

async fn start_deployment(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: StartDeploymentPath {
			workspace_id,
			deployment_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<StartDeploymentRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Start deployment", request_id);

	// TODO - fix this in axum
	let ip_address = routes::get_request_ip_address(&context);

	let user_id = token_data.user_id().clone();

	let login_id = token_data.login_id().clone();

	// start the container running the image, if doesn't exist
	log::trace!(
		"request_id: {} - Starting deployment with id: {}",
		request_id,
		deployment_id
	);
	let (deployment, workspace_id, _, deployment_running_details) =
		service::get_full_deployment_config(
			&mut connection,
			&deployment_id,
			&request_id,
		)
		.await?;
	let now = Utc::now();

	if let DeploymentRegistry::PatrRegistry { repository_id, .. } =
		&deployment.registry
	{
		let digest = db::get_latest_digest_for_docker_repository(
			&mut connection,
			repository_id,
		)
		.await?;

		if let Some(digest) = digest {
			// Check if digest is already in deployment_deploy_history table
			let deployment_deploy_history =
				db::get_deployment_image_digest_by_digest(
					&mut connection,
					&digest,
				)
				.await?;

			// If not, add it to the table
			if deployment_deploy_history.is_none() {
				db::add_digest_to_deployment_deploy_history(
					&mut connection,
					&deployment_id,
					repository_id,
					&digest,
					&now,
				)
				.await?;
			}
		}
	}

	log::trace!("request_id: {} - Start deployment", request_id);
	service::start_deployment(
		&mut connection,
		&workspace_id,
		&deployment_id,
		&deployment,
		&deployment_running_details,
		&user_id,
		&login_id,
		&ip_address,
		&DeploymentMetadata::Start {},
		&now,
		&config,
		&request_id,
	)
	.await?;

	connection.commit().await?;

	service::queue_check_and_update_deployment_status(
		&workspace_id,
		&deployment_id,
		&config,
		&request_id,
	)
	.await?;

	Ok(())
}

async fn stop_deployment(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: StopDeploymentPath {
			workspace_id,
			deployment_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<StopDeploymentRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	let ip_address = routes::get_request_ip_address(&context);

	let user_id = token_data.user_id().clone();

	let login_id = token_data.login_id().clone();

	log::trace!("request_id: {} - Getting deployment id from db", request_id);
	let deployment = db::get_deployment_by_id(&mut connection, &deployment_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	log::trace!(
		"request_id: {} - Stopping the deployment {}",
		request_id,
		deployment_id
	);

	service::stop_deployment(
		&mut connection,
		&deployment.workspace_id,
		&deployment_id,
		&deployment.region,
		&user_id,
		&login_id,
		&ip_address,
		&config,
		&request_id,
	)
	.await?;

	Ok(());
}

async fn revert_deployment(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path:
			RevertDeploymentPath {
				workspace_id,
				deployment_id,
				digest,
			},
		query: (),
		body: (),
	}: DecodedRequest<RevertDeploymentRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Getting deployment id from db", request_id);
	let deployment = db::get_deployment_by_id(&mut connection, &deployment_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	log::trace!(
		"request_id: {} - Getting info digest info from db",
		request_id
	);

	// Check if the digest is present or not in the deployment_deploy_history
	// table
	db::get_deployment_image_digest_by_digest(&mut connection, &digest)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	db::update_current_live_digest_for_deployment(
		&mut connection,
		&deployment.id,
		&digest,
	)
	.await?;

	let (deployment, workspace_id, _, deployment_running_details) =
		service::get_full_deployment_config(
			&mut connection,
			&deployment.id,
			&request_id,
		)
		.await?;

	log::trace!(
		"request_id: {} - queuing revert the deployment request",
		request_id
	);

	let (image_name, _) =
		service::get_image_name_and_digest_for_deployment_image(
			&mut connection,
			&deployment.registry,
			&deployment.image_tag,
			&config,
			&request_id,
		)
		.await?;

	db::update_deployment_status(
		&mut connection,
		&deployment_id,
		&DeploymentStatus::Deploying,
	)
	.await?;

	service::update_deployment_image(
		&mut connection,
		&workspace_id,
		&deployment_id,
		&deployment.name,
		&deployment.registry,
		&digest,
		&deployment.image_tag,
		&image_name,
		&deployment.region,
		&deployment.machine_type,
		&deployment_running_details,
		&config,
		&request_id,
	)
	.await?;

	connection.commit().await?;

	service::queue_check_and_update_deployment_status(
		&workspace_id,
		&deployment_id,
		&config,
		&request_id,
	)
	.await?;

	Ok(())
}

async fn get_logs(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GetDeploymentLogsPath {
			workspace_id,
			deployment_id,
		},
		query: (),
		body: GetDeploymentLogsRequest { end_time, limit },
	}: DecodedRequest<GetDeploymentLogsRequest>,
) -> Result<GetDeploymentLogsResponse, Error> {
	let request_id = Uuid::new_v4();

	let deployment = db::get_deployment_by_id(&mut connection, &deployment_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	if !service::is_deployed_on_patr_cluster(
		&mut connection,
		&deployment.region,
	)
	.await?
	{
		return Err(ErrorType::FeatureNotSupportedForCustomCluster);
	}

	let end_time = end_time
		.map(|DateTime(end_time)| end_time)
		.unwrap_or_else(Utc::now);

	// Loki query limit to 721h in time range
	let start_time = end_time - Duration::days(30);

	log::trace!("request_id: {} - Getting logs", request_id);
	let logs = service::get_deployment_container_logs(
		&mut connection,
		&deployment_id,
		&start_time,
		&end_time,
		limit.unwrap_or(100),
		&config,
		&request_id,
	)
	.await?;

	Ok(GetDeploymentLogsResponse { logs })
}

async fn delete_deployment(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: DeleteDeploymentPath {
			workspace_id,
			deployment_id,
		},
		query: (),
		body: DeleteDeploymentRequest { hard_delete },
	}: DecodedRequest<DeleteDeploymentRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	let ip_address = routes::get_request_ip_address(&context);

	let user_id = token_data.user_id().clone();

	let login_id = token_data.login_id().clone();

	log::trace!(
		"request_id: {} - Deleting the deployment with id: {}",
		request_id,
		deployment_id
	);
	// stop and delete the container running the image, if it exists
	let deployment = db::get_deployment_by_id(&mut connection, &deployment_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	if service::is_deployed_on_patr_cluster(&mut connection, &deployment.region)
		.await?
	{
		db::stop_deployment_usage_history(
			&mut connection,
			&deployment_id,
			&Utc::now(),
		)
		.await?;

		let volumes =
			db::get_all_deployment_volumes(&mut connection, &deployment.id)
				.await?;

		for volume in volumes {
			db::stop_volume_usage_history(
				&mut connection,
				&volume.volume_id,
				&Utc::now(),
			)
			.await?;
		}
	}

	log::trace!("request_id: {} - Checking is any managed url is used by the deployment: {}", request_id, deployment_id);
	let managed_url = db::get_all_managed_urls_for_deployment(
		&mut connection,
		&deployment_id,
		&workspace_id,
	)
	.await?;

	if !managed_url.is_empty() {
		log::trace!(
			"deployment: {} - is using managed_url. Cannot delete it",
			deployment_id
		);
		return ErrorType::ResourceInUse;
	}

	let region = db::get_region_by_id(&mut connection, &deployment.region)
		.await?
		.ok_or_else(|| ErrorType::internal_error());

	let delete_k8s_resource = if region.is_patr_region() {
		true
	} else {
		hard_delete
	};

	log::trace!("request_id: {} - Deleting deployment", request_id);
	service::delete_deployment(
		&mut connection,
		&deployment.workspace_id,
		&deployment_id,
		&deployment.region,
		Some(&user_id),
		Some(&login_id),
		&ip_address,
		false,
		delete_k8s_resource,
		&config,
		&request_id,
	)
	.await?;

	service::get_internal_metrics(
		&mut connection,
		"A deployment has been deleted",
	)
	.await;

	// Commiting transaction so that even if the mailing function fails the
	// resource should be deleted
	connection.commit().await?;

	service::resource_delete_action_email(
		&mut connection,
		&deployment.name,
		&deployment.workspace_id,
		&ResourceType::Deployment,
		&user_id,
	)
	.await?;

	Ok(());
}

async fn update_deployment(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: UpdateDeploymentPath {
			workspace_id,
			deployment_id,
		},
		query: (),
		body:
			UpdateDeploymentRequest {
				name,
				machine_type,
				deploy_on_push,
				min_horizontal_scale,
				max_horizontal_scale,
				ports,
				environment_variables,
				startup_probe,
				liveness_probe,
				config_mounts,
				volumes,
			},
	}: DecodedRequest<UpdateDeploymentRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	// workspace_id in UpdateDeploymentRequest struct parsed as null uuid(0..0),
	// hence taking the value here which will be same

	log::trace!(
		"request_id: {} - Updating deployment with id: {}",
		request_id,
		deployment_id
	);

	// Is any one value present?
	if name.is_none() &&
		machine_type.is_none() &&
		deploy_on_push.is_none() &&
		min_horizontal_scale.is_none() &&
		max_horizontal_scale.is_none() &&
		ports.is_none() &&
		environment_variables.is_none() &&
		startup_probe.is_none() &&
		liveness_probe.is_none() &&
		config_mounts.is_none() &&
		volumes.is_none()
	{
		return Err(ErrorType::WrongParameters);
	}

	service::update_deployment(
		&mut connection,
		&workspace_id,
		&deployment_id,
		name,
		machine_type.as_ref(),
		deploy_on_push,
		min_horizontal_scale,
		max_horizontal_scale,
		ports
			.map(|ports| {
				ports
					.into_iter()
					.map(|(k, v)| (k.value(), v))
					.collect::<BTreeMap<_, _>>()
			})
			.as_ref(),
		environment_variables.as_ref(),
		startup_probe.as_ref(),
		liveness_probe.as_ref(),
		config_mounts.as_ref(),
		volumes.as_ref(),
		&config,
		&request_id,
	)
	.await?;

	connection.commit().await?;

	service::queue_check_and_update_deployment_status(
		&workspace_id,
		&deployment_id,
		&config,
		&request_id,
	)
	.await?;

	Ok(())
}

async fn list_linked_urls(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: ListLinkedURLsPath {
			workspace_id,
			deployment_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<ListLinkedURLsRequest>,
) -> Result<ListLinkedURLsResponse, Error> {
	let urls = db::get_all_managed_urls_for_deployment(
		&mut connection,
		&deployment_id,
		&workspace_id,
	)
	.await?
	.into_iter()
	.filter_map(|url| {
		Some(ManagedUrl {
			id: url.id,
			sub_domain: url.sub_domain,
			domain_id: url.domain_id,
			path: url.path,
			url_type: match url.url_type {
				DbManagedUrlType::ProxyToDeployment => {
					ManagedUrlType::ProxyDeployment {
						deployment_id: url.deployment_id?,
						port: url.port? as u16,
					}
				}
				DbManagedUrlType::ProxyToStaticSite => {
					ManagedUrlType::ProxyStaticSite {
						static_site_id: url.static_site_id?,
					}
				}
				DbManagedUrlType::ProxyUrl => ManagedUrlType::ProxyUrl {
					url: url.url?,
					http_only: url.http_only?,
				},
				DbManagedUrlType::Redirect => ManagedUrlType::Redirect {
					url: url.url?,
					permanent_redirect: url.permanent_redirect?,
					http_only: url.http_only?,
				},
			},
			is_configured: url.is_configured,
		})
	})
	.collect();

	Ok(ListLinkedURLsResponse { urls })
}

async fn get_deployment_metrics(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: GetDeploymentMetricsPath {
			workspace_id,
			deployment_id,
		},
		query: (),
		body: GetDeploymentMetricsRequest { start_time, step },
	}: DecodedRequest<GetDeploymentMetricsRequest>,
) -> Result<GetDeploymentMetricsResponse, Error> {
	let request_id = Uuid::new_v4();

	let deployment = db::get_deployment_by_id(&mut connection, &deployment_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	let region = db::get_region_by_id(&mut connection, &deployment.region)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	if region.is_byoc_region() {
		return Err(ErrorType::FeatureNotSupportedForCustomCluster);
	}

	log::trace!(
		"request_id: {} - Getting deployment metrics for deployment: {}",
		request_id,
		deployment_id
	);
	let start_time = Utc::now() -
		match start_time.parse::<Interval>().unwrap_or(Interval::Hour) {
			Interval::Hour => Duration::hours(1),
			Interval::Day => Duration::days(1),
			Interval::Week => Duration::weeks(1),
			Interval::Month => Duration::days(30),
			Interval::Year => Duration::days(365),
		};

	let step = step.parse::<Step>().unwrap_or(Step::TenMinutes);

	let deployment_metrics = service::get_deployment_metrics(
		&deployment_id,
		&config,
		&start_time,
		&Utc::now(),
		&step.to_string(),
		&request_id,
	)
	.await?;

	Ok(GetDeploymentMetricsResponse {
		metrics: deployment_metrics,
	})
}

async fn get_build_logs(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path:
			GetDeploymentBuildLogsPath {
				workspace_id,
				deployment_id,
			},
		query: (),
		body: GetDeploymentBuildLogsRequest { start_time },
	}: DecodedRequest<GetDeploymentBuildLogsRequest>,
) -> Result<GetDeploymentBuildLogsResponse, Error> {
	let request_id = Uuid::new_v4();

	let start_time = Utc::now() -
		match start_time.unwrap_or(Interval::Hour) {
			Interval::Hour => Duration::hours(1),
			Interval::Day => Duration::days(1),
			Interval::Week => Duration::weeks(1),
			Interval::Month => Duration::days(30),
			Interval::Year => Duration::days(365),
		};

	log::trace!("request_id: {} - Getting build logs", request_id);
	// stop the running container, if it exists
	let logs = service::get_deployment_build_logs(
		&workspace_id,
		&deployment_id,
		&start_time,
		&Utc::now(),
		&config,
		&request_id,
	)
	.await?
	.into_iter()
	.map(|build_log| BuildLog {
		timestamp: build_log
			.metadata
			.creation_timestamp
			.map(|Time(timestamp)| timestamp.timestamp_millis() as u64),
		reason: build_log.reason,
		message: build_log.message,
	})
	.collect();
	Ok(GetDeploymentBuildLogsResponse { logs })
}

async fn get_build_events(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: GetDeploymentEventsPath {
			workspace_id,
			deployment_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<GetDeploymentEventsRequest>,
) -> Result<GetDeploymentEventsResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {} - Checking if the deployment exists or not",
		request_id
	);

	db::get_deployment_by_id(&mut connection, &deployment_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	log::trace!(
		"request_id: {} - Getting the build events from the database",
		request_id
	);
	let build_events =
		db::get_build_events_for_deployment(&mut connection, &deployment_id)
			.await?
			.into_iter()
			.map(|event| WorkspaceAuditLog {
				id: event.id,
				date: DateTime(event.date),
				ip_address: event.ip_address,
				workspace_id: event.workspace_id,
				user_id: event.user_id,
				login_id: event.login_id,
				resource_id: event.resource_id,
				action: event.action,
				request_id: event.request_id,
				metadata: event.metadata,
				patr_action: event.patr_action,
				request_success: event.success,
			})
			.collect();

	log::trace!(
		"request_id: {} - Build events successfully retreived",
		request_id
	);
	Ok(GetDeploymentEventsResponse { logs: build_events })
}
