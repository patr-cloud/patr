use std::collections::BTreeMap;

use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::{
		infrastructure::{
			deployment::{
				BuildLog,
				CreateDeploymentRequest,
				CreateDeploymentResponse,
				DeleteDeploymentResponse,
				Deployment,
				DeploymentDeployHistory,
				DeploymentRegistry,
				DeploymentStatus,
				GetDeploymentBuildLogsRequest,
				GetDeploymentBuildLogsResponse,
				GetDeploymentEventsResponse,
				GetDeploymentInfoResponse,
				GetDeploymentLogsRequest,
				GetDeploymentLogsResponse,
				GetDeploymentMetricsResponse,
				Interval,
				ListDeploymentHistoryResponse,
				ListDeploymentsResponse,
				ListLinkedURLsResponse,
				PatrRegistry,
				RevertDeploymentResponse,
				StartDeploymentResponse,
				Step,
				StopDeploymentResponse,
				UpdateDeploymentRequest,
				UpdateDeploymentResponse,
			},
			managed_urls::{ManagedUrl, ManagedUrlType},
		},
		WorkspaceAuditLog,
	},
	utils::{constants, DateTime, Uuid},
};
use chrono::{Duration, Utc};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;

use crate::{
	app::{create_eve_app, App},
	db::{self, ManagedUrlType as DbManagedUrlType},
	error,
	models::{
		rbac::{self, permissions},
		DeploymentMetadata,
	},
	pin_fn,
	routes::api_patr_cloud,
	service,
	utils::{
		constants::request_keys,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};

/// # Description
/// This function is used to create a sub app for every endpoint listed. It
/// creates an eve app which binds the endpoint with functions.
///
/// # Arguments
/// * `app` - an object of type [`App`] which contains all the configuration of
///   api including the
/// database connections.
///
/// # Returns
/// this function returns `EveApp<EveContext, EveMiddleware, App, ErrorData>`
/// containing context, middleware, object of [`App`] and Error
///
/// [`App`]: App
pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut app = create_eve_app(app);

	// List all deployments
	app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::LIST,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(list_deployments)),
		],
	);

	app.get(
		"/:deploymentId/deploy-history",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::INFO,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let deployment_id =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
					)
					.await?
					.filter(|resource| resource.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(list_deployment_history)),
		],
	);

	// Create a new deployment
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::CREATE,
				closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(create_deployment)),
		],
	);

	// Get info about a deployment
	app.get(
		"/:deploymentId/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::INFO,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_deployment_info)),
		],
	);

	// start a deployment
	app.post(
		"/:deploymentId/start",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::EDIT,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(start_deployment)),
		],
	);

	// stop the deployment
	app.post(
		"/:deploymentId/stop",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::EDIT,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(stop_deployment)),
		],
	);

	// revert the deployment
	app.post(
		"/:deploymentId/deploy-history/:digest/revert",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::EDIT,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(revert_deployment)),
		],
	);

	// get logs for the deployment
	app.get(
		"/:deploymentId/logs",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::INFO,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_logs)),
		],
	);

	// Delete a deployment
	app.delete(
		"/:deploymentId/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::DELETE,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(delete_deployment)),
		],
	);

	// Update a deployment
	app.patch(
		"/:deploymentId/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::EDIT,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(update_deployment)),
		],
	);

	// List all linked URLs of a deployment
	app.get(
		"/:deploymentId/managed-urls",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::INFO,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(list_linked_urls)),
		],
	);

	// get all deployment metrics
	app.get(
		"/:deploymentId/metrics",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::INFO,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_deployment_metrics)),
		],
	);

	app.get(
		"/:deploymentId/build-logs",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::LIST,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_build_logs)),
		],
	);

	app.get(
		"/:deploymentId/events",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::deployment::LIST,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_build_events)),
		],
	);

	app
}

/// # Description
/// This function is used to list of all the deployments present with the user
/// required inputs:
/// workspaceId in url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success:
///    deployments: []
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn list_deployments(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Listing deployments", request_id);

	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	log::trace!(
		"request_id: {} - Getting deployments from database",
		request_id
	);
	let deployments = db::get_deployments_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.filter_map(|deployment| {
		Some(Deployment {
			id: deployment.id,
			name: deployment.name,
			registry: if deployment.registry == constants::PATR_REGISTRY {
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

	context.success(ListDeploymentsResponse { deployments });
	Ok(context)
}

/// # Description
/// This function is used to list of image digest for the deployments with user
/// required inputs:
/// workspaceId and deploymentId in url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success:
///    deployments: []
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn list_deployment_history(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Listing deployments", request_id);

	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();
	let deployment = db::get_deployment_by_id(
		context.get_database_connection(),
		&deployment_id,
	)
	.await?;

	log::trace!(
		"request_id: {} - Getting deployment image digest history from database",
		request_id
	);
	let deploys = db::get_all_digest_for_deployment(
		context.get_database_connection(),
		&deployment_id,
	)
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

	// Check if no image is pushed for this deployment_id
	if deployment.is_none() {
		return Error::as_result()
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	}

	context.success(ListDeploymentHistoryResponse { deploys });
	Ok(context)
}

/// # Description
/// This function is used to create a new deployment
/// required inputs
/// auth token in the header
/// workspace id in parameter
/// ```
/// {
///    name: ,
///    registry: ,
///    repositoryId: ,
///    imageName: ,
///    imageTag: ,
///    region: ,
///    domainName: ,
///    horizontalScale: ,
///    machineType:
/// }
/// ```
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success:
///    deploymentId:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn create_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Creating deployment", request_id);
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let ip_address = api_patr_cloud::get_request_ip_address(&context);

	let user_id = context.get_token_data().unwrap().user.id.clone();
	let login_id = context.get_token_data().unwrap().login_id.clone();

	let CreateDeploymentRequest {
		workspace_id: _,
		name,
		registry,
		image_tag,
		region,
		machine_type,
		running_details: deployment_running_details,
		deploy_on_create,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let name = name.trim();
	let image_tag = image_tag.trim();

	let config = context.get_state().config.clone();

	log::trace!(
		"request_id: {} - Creating the deployment in workspace",
		request_id
	);

	let id = service::create_deployment_in_workspace(
		context.get_database_connection(),
		&workspace_id,
		name,
		&registry,
		image_tag,
		&region,
		&machine_type,
		&deployment_running_details,
		&request_id,
	)
	.await?;

	let audit_log_id = db::generate_new_workspace_audit_log_id(
		context.get_database_connection(),
	)
	.await?;

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
		running_details: deployment_running_details.clone(),
	})?;

	db::create_workspace_audit_log(
		context.get_database_connection(),
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

	context.commit_database_transaction().await?;

	if deploy_on_create {
		let mut is_deployed = false;
		if let DeploymentRegistry::PatrRegistry { repository_id, .. } =
			&registry
		{
			let digest = db::get_latest_digest_for_docker_repository(
				context.get_database_connection(),
				repository_id,
			)
			.await?;

			if let Some(digest) = digest {
				db::add_digest_to_deployment_deploy_history(
					context.get_database_connection(),
					&id,
					repository_id,
					&digest,
					&now,
				)
				.await?;

				db::update_current_live_digest_for_deployment(
					context.get_database_connection(),
					&id,
					&digest,
				)
				.await?;

				if db::get_docker_repository_tag_details(
					context.get_database_connection(),
					repository_id,
					image_tag,
				)
				.await?
				.is_some()
				{
					service::start_deployment(
						context.get_database_connection(),
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
						&deployment_running_details,
						&user_id,
						&login_id,
						&ip_address,
						&DeploymentMetadata::Start {},
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
				context.get_database_connection(),
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
				&deployment_running_details,
				&user_id,
				&login_id,
				&ip_address,
				&DeploymentMetadata::Start {},
				&config,
				&request_id,
			)
			.await?;
			is_deployed = true;
		}

		if is_deployed {
			context.commit_database_transaction().await?;

			service::queue_check_and_update_deployment_status(
				&workspace_id,
				&id,
				&config,
				&request_id,
			)
			.await?;
		}
	}

	let _ = service::get_internal_metrics(
		context.get_database_connection(),
		"A new deployment has been created",
	)
	.await;

	context.success(CreateDeploymentResponse { id });
	Ok(context)
}

/// # Description
/// This function is used to get the information about a specific deployment
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// DeploymentId in url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///     success: true or false,
///     deployment:
///     {
///         id: ,
///         name: ,
///         registry: ,
///         imageName: ,
///         imageTag: ,
///         domainName: ,
///         horizontalScale: ,
///         machineType:
///     }
/// }
/// ```
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_deployment_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();

	log::trace!(
		"request_id: {} - Getting deployment details from the database for deployment: {}",
		request_id,
		deployment_id,
	);
	let (deployment, _, _, running_details) =
		service::get_full_deployment_config(
			context.get_database_connection(),
			&deployment_id,
			&request_id,
		)
		.await?;

	context.success(GetDeploymentInfoResponse {
		deployment,
		running_details,
	});
	Ok(context)
}

/// # Description
/// This function is used to start a deployment
/// required inputs:
/// deploymentId in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the next
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn start_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Start deployment", request_id);

	let ip_address = api_patr_cloud::get_request_ip_address(&context);

	let user_id = context.get_token_data().unwrap().user.id.clone();

	let login_id = context.get_token_data().unwrap().login_id.clone();

	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();

	// start the container running the image, if doesn't exist
	let config = context.get_state().config.clone();
	log::trace!(
		"request_id: {} - Starting deployment with id: {}",
		request_id,
		deployment_id
	);
	let (deployment, workspace_id, _, deployment_running_details) =
		service::get_full_deployment_config(
			context.get_database_connection(),
			&deployment_id,
			&request_id,
		)
		.await?;
	let now = Utc::now();

	if let DeploymentRegistry::PatrRegistry { repository_id, .. } =
		&deployment.registry
	{
		let digest = db::get_latest_digest_for_docker_repository(
			context.get_database_connection(),
			repository_id,
		)
		.await?;

		if let Some(digest) = digest {
			// Check if digest is already in deployment_deploy_history table
			let deployment_deploy_history =
				db::get_deployment_image_digest_by_digest(
					context.get_database_connection(),
					&digest,
				)
				.await?;

			// If not, add it to the table
			if deployment_deploy_history.is_none() {
				db::add_digest_to_deployment_deploy_history(
					context.get_database_connection(),
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
		context.get_database_connection(),
		&workspace_id,
		&deployment_id,
		&deployment,
		&deployment_running_details,
		&user_id,
		&login_id,
		&ip_address,
		&DeploymentMetadata::Start {},
		&config,
		&request_id,
	)
	.await?;

	context.commit_database_transaction().await?;

	service::queue_check_and_update_deployment_status(
		&workspace_id,
		&deployment_id,
		&config,
		&request_id,
	)
	.await?;

	context.success(StartDeploymentResponse {});
	Ok(context)
}

/// # Description
/// This function is used to stop a deployment
/// required inputs:
/// deploymentId in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the next
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn stop_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();

	let ip_address = api_patr_cloud::get_request_ip_address(&context);

	let user_id = context.get_token_data().unwrap().user.id.clone();

	let login_id = context.get_token_data().unwrap().login_id.clone();

	let config = context.get_state().config.clone();
	log::trace!("request_id: {} - Getting deployment id from db", request_id);
	let deployment = db::get_deployment_by_id(
		context.get_database_connection(),
		&deployment_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	log::trace!(
		"request_id: {} - Stopping the deployment {}",
		request_id,
		deployment_id
	);
	service::stop_deployment(
		context.get_database_connection(),
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

	context.success(StopDeploymentResponse {});
	Ok(context)
}

/// # Description
/// This function is used to revert a deployment to specific image hash
/// required inputs:
/// deploymentId and digest in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the next
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn revert_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();
	let image_digest =
		context.get_param(request_keys::DIGEST).unwrap().to_string();

	let config = context.get_state().config.clone();
	log::trace!("request_id: {} - Getting deployment id from db", request_id);
	let deployment = db::get_deployment_by_id(
		context.get_database_connection(),
		&deployment_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	log::trace!(
		"request_id: {} - Getting info digest info from db",
		request_id
	);

	// Check if the digest is present or not in the deployment_deploy_history
	// table
	db::get_deployment_image_digest_by_digest(
		context.get_database_connection(),
		&image_digest,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	db::update_current_live_digest_for_deployment(
		context.get_database_connection(),
		&deployment.id,
		&image_digest,
	)
	.await?;

	let (deployment, workspace_id, _, deployment_running_details) =
		service::get_full_deployment_config(
			context.get_database_connection(),
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
			context.get_database_connection(),
			&deployment.registry,
			&deployment.image_tag,
			&config,
			&request_id,
		)
		.await?;

	db::update_deployment_status(
		context.get_database_connection(),
		&deployment_id,
		&DeploymentStatus::Deploying,
	)
	.await?;

	service::update_deployment_image(
		context.get_database_connection(),
		&workspace_id,
		&deployment_id,
		&deployment.name,
		&deployment.registry,
		&image_digest,
		&deployment.image_tag,
		&image_name,
		&deployment.region,
		&deployment.machine_type,
		&deployment_running_details,
		&config,
		&request_id,
	)
	.await?;

	context.commit_database_transaction().await?;

	service::queue_check_and_update_deployment_status(
		&workspace_id,
		&deployment_id,
		&config,
		&request_id,
	)
	.await?;

	context.success(RevertDeploymentResponse {});
	Ok(context)
}

/// # Description
/// This function is used to get the logs of a deployment
/// required inputs:
/// deploymentId in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the next
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_logs(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let GetDeploymentLogsRequest { start_time, .. } = context
		.get_query_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let request_id = Uuid::new_v4();

	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();

	let deployment = db::get_deployment_by_id(
		context.get_database_connection(),
		&deployment_id,
	)
	.await?
	.status(500)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	if !service::is_deployed_on_patr_cluster(
		context.get_database_connection(),
		&deployment.region,
	)
	.await?
	{
		return Err(Error::empty().status(500).body(
			error!(FEATURE_NOT_SUPPORTED_FOR_CUSTOM_CLUSTER).to_string(),
		));
	}

	let config = context.get_state().config.clone();

	let start_time = Utc::now() -
		match start_time.unwrap_or(Interval::Hour) {
			Interval::Hour => Duration::hours(1),
			Interval::Day => Duration::days(1),
			Interval::Week => Duration::weeks(1),
			Interval::Month => Duration::days(30),
			Interval::Year => Duration::days(365),
		};

	log::trace!("request_id: {} - Getting logs", request_id);
	// stop the running container, if it exists
	let logs = service::get_deployment_container_logs(
		context.get_database_connection(),
		&deployment_id,
		&start_time,
		&Utc::now(),
		&config,
		&request_id,
	)
	.await?;

	context.success(GetDeploymentLogsResponse { logs });
	Ok(context)
}

/// # Description
/// This function is used to delete deployment
/// required inputs:
/// deploymentId in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn delete_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();

	let ip_address = api_patr_cloud::get_request_ip_address(&context);

	let user_id = context.get_token_data().unwrap().user.id.clone();

	let login_id = context.get_token_data().unwrap().login_id.clone();

	log::trace!(
		"request_id: {} - Deleting the deployment with id: {}",
		request_id,
		deployment_id
	);
	// stop and delete the container running the image, if it exists
	let config = context.get_state().config.clone();
	let deployment = db::get_deployment_by_id(
		context.get_database_connection(),
		&deployment_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	if service::is_deployed_on_patr_cluster(
		context.get_database_connection(),
		&deployment.region,
	)
	.await?
	{
		db::stop_deployment_usage_history(
			context.get_database_connection(),
			&deployment_id,
			&Utc::now(),
		)
		.await?;
	}

	log::trace!("request_id: {} - Deleting deployment", request_id);
	service::delete_deployment(
		context.get_database_connection(),
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

	let _ = service::get_internal_metrics(
		context.get_database_connection(),
		"A deployment has been deleted",
	)
	.await;

	context.success(DeleteDeploymentResponse {});
	Ok(context)
}

/// # Description
/// This function is used to update deployment
/// required inputs:
/// deploymentId in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn update_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();

	log::trace!(
		"{} - Updating deployment with id: {}",
		request_id,
		deployment_id
	);
	let UpdateDeploymentRequest {
		workspace_id: _,
		deployment_id: _,
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
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let name = name.as_ref().map(|name| name.trim());

	let user_id = context.get_token_data().unwrap().user.id.clone();

	let login_id = context.get_token_data().unwrap().login_id.clone();

	let ip_address = api_patr_cloud::get_request_ip_address(&context);

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
		config_mounts.is_none()
	{
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	let config = context.get_state().config.clone();

	let metadata = DeploymentMetadata::Update {
		name: name.map(|n| n.to_string()),
		machine_type: machine_type.clone(),
		deploy_on_push,
		min_horizontal_scale,
		max_horizontal_scale,
		ports: ports.clone(),
		environment_variables: environment_variables.clone(),
		startup_probe: startup_probe.clone(),
		liveness_probe: liveness_probe.clone(),
	};

	service::update_deployment(
		context.get_database_connection(),
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
		&request_id,
	)
	.await?;

	context.commit_database_transaction().await?;

	let (deployment, workspace_id, _, deployment_running_details) =
		service::get_full_deployment_config(
			context.get_database_connection(),
			&deployment_id,
			&request_id,
		)
		.await?;

	match &deployment.status {
		DeploymentStatus::Stopped |
		DeploymentStatus::Deleted |
		DeploymentStatus::Created => {
			// Don't update deployments that are explicitly stopped or deleted
		}
		_ => {
			let current_time = Utc::now();
			if service::is_deployed_on_patr_cluster(
				context.get_database_connection(),
				&deployment.region,
			)
			.await?
			{
				db::stop_deployment_usage_history(
					context.get_database_connection(),
					&deployment_id,
					&current_time,
				)
				.await?;
				db::start_deployment_usage_history(
					context.get_database_connection(),
					&workspace_id,
					&deployment_id,
					&deployment.machine_type,
					deployment_running_details.min_horizontal_scale as i32,
					&current_time,
				)
				.await?;
			}

			service::start_deployment(
				context.get_database_connection(),
				&workspace_id,
				&deployment_id,
				&deployment,
				&deployment_running_details,
				&user_id,
				&login_id,
				&ip_address,
				&metadata,
				&config,
				&request_id,
			)
			.await?;

			context.commit_database_transaction().await?;

			service::queue_check_and_update_deployment_status(
				&workspace_id,
				&deployment_id,
				&config,
				&request_id,
			)
			.await?;
		}
	}

	context.success(UpdateDeploymentResponse {});
	Ok(context)
}

/// # Description
/// This function is used to list linked urls for the deployment
/// required inputs:
/// deploymentId in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn list_linked_urls(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();
	let workspace_id = Uuid::parse_str(
		context.get_param(request_keys::WORKSPACE_ID).unwrap(),
	)?;

	let urls = db::get_all_managed_urls_for_deployment(
		context.get_database_connection(),
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
				DbManagedUrlType::ProxyUrl => {
					ManagedUrlType::ProxyUrl { url: url.url? }
				}
				DbManagedUrlType::Redirect => {
					ManagedUrlType::Redirect { url: url.url? }
				}
			},
			is_configured: url.is_configured,
		})
	})
	.collect();

	context.success(ListLinkedURLsResponse { urls });
	Ok(context)
}

/// # Description
/// This function is used to fetch all deployment metrics (cpu usage, memory
/// usage, etc) required inputs:
/// deploymentId in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_deployment_metrics(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();

	let deployment = db::get_deployment_by_id(
		context.get_database_connection(),
		&deployment_id,
	)
	.await?
	.status(500)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	if !service::is_deployed_on_patr_cluster(
		context.get_database_connection(),
		&deployment.region,
	)
	.await?
	{
		return Err(Error::empty().status(500).body(
			error!(FEATURE_NOT_SUPPORTED_FOR_CUSTOM_CLUSTER).to_string(),
		));
	}

	log::trace!(
		"request_id: {} - Getting deployment metrics for deployment: {}",
		request_id,
		deployment_id
	);
	let start_time = Utc::now() -
		match context
			.get_request()
			.get_query()
			.get(request_keys::START_TIME)
			.and_then(|value| value.parse::<Interval>().ok())
			.unwrap_or(Interval::Hour)
		{
			Interval::Hour => Duration::hours(1),
			Interval::Day => Duration::days(1),
			Interval::Week => Duration::weeks(1),
			Interval::Month => Duration::days(30),
			Interval::Year => Duration::days(365),
		};

	let step = context
		.get_request()
		.get_query()
		.get(request_keys::INTERVAL)
		.and_then(|value| value.parse::<Step>().ok())
		.unwrap_or(Step::TenMinutes);

	let config = context.get_state().config.clone();

	let deployment_metrics = service::get_deployment_metrics(
		&deployment_id,
		&config,
		&start_time,
		&Utc::now(),
		&step.to_string(),
		&request_id,
	)
	.await?;

	context.success(GetDeploymentMetricsResponse {
		metrics: deployment_metrics,
	});
	Ok(context)
}

/// # Description
/// This function is used to get the build logs for a deployment
/// required inputs:
/// deploymentId in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_build_logs(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let config = context.get_state().config.clone();

	let GetDeploymentBuildLogsRequest { start_time, .. } = context
		.get_query_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

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
	context.success(GetDeploymentBuildLogsResponse { logs });
	Ok(context)
}

/// # Description
/// This function is used to get the build events for a deployment
/// required inputs:
/// deploymentId in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_build_events(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();

	log::trace!(
		"request_id: {} - Checking if the deployment exists or not",
		request_id
	);
	let _ = db::get_deployment_by_id(
		context.get_database_connection(),
		&deployment_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	log::trace!(
		"request_id: {} - Getting the build events from the database",
		request_id
	);
	let build_events = db::get_build_events_for_deployment(
		context.get_database_connection(),
		&deployment_id,
	)
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
	context.success(GetDeploymentEventsResponse { logs: build_events });
	Ok(context)
}
