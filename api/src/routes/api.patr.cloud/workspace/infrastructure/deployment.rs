use std::collections::{BTreeMap, HashMap};

use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::{
		infrastructure::{
			deployment::{
				BuildLog,
				CreateDeploymentRequest,
				CreateDeploymentResponse,
				DeleteDeploymentRequest,
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
use eve_rs::{App as EveApp, AsError, Context, Error as _, NextHandler};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;

use crate::{
	app::{create_eve_app, App},
	db::{self, ManagedUrlType as DbManagedUrlType},
	error,
	models::{
		cloudflare::deployment,
		rbac::{self, permissions},
		DeploymentMetadata,
		ResourceType,
	},
	pin_fn,
	routes,
	service,
	utils::{
		constants::{logs::PATR_CLUSTER_TENANT_ID, request_keys},
		Error,
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
) -> EveApp<EveContext, EveMiddleware, App, Error> {
	let mut app = create_eve_app(app);

	// List all deployments
	app.get(
		"/",
		[
			EveMiddleware::WorkspaceMemberAuthenticator {
				is_api_token_allowed: true,
				requested_workspace: closure_as_pinned_box!(|context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					Ok((context, workspace_id))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(list_deployments)),
		],
	);

	app.get(
		"/:deploymentId/deploy-history",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::deployment::INFO,
				resource: closure_as_pinned_box!(|mut context| {
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
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(list_deployment_history)),
		],
	);

	// Create a new deployment
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::deployment::CREATE,
				resource: closure_as_pinned_box!(|mut context| {
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
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(create_deployment)),
		],
	);

	// Get info about a deployment
	app.get(
		"/:deploymentId/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::deployment::INFO,
				resource: closure_as_pinned_box!(|mut context| {
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
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(get_deployment_info)),
		],
	);

	// start a deployment
	app.post(
		"/:deploymentId/start",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::deployment::EDIT,
				resource: closure_as_pinned_box!(|mut context| {
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
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(start_deployment)),
		],
	);

	// stop the deployment
	app.post(
		"/:deploymentId/stop",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::deployment::EDIT,
				resource: closure_as_pinned_box!(|mut context| {
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
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(stop_deployment)),
		],
	);

	// revert the deployment
	app.post(
		"/:deploymentId/deploy-history/:digest/revert",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::deployment::EDIT,
				resource: closure_as_pinned_box!(|mut context| {
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
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(revert_deployment)),
		],
	);

	// get logs for the deployment
	app.get(
		"/:deploymentId/logs",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::deployment::INFO,
				resource: closure_as_pinned_box!(|mut context| {
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
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(get_logs)),
		],
	);

	// Delete a deployment
	app.delete(
		"/:deploymentId/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::deployment::DELETE,
				resource: closure_as_pinned_box!(|mut context| {
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
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(delete_deployment)),
		],
	);

	// Update a deployment
	app.patch(
		"/:deploymentId/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::deployment::EDIT,
				resource: closure_as_pinned_box!(|mut context| {
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
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(update_deployment)),
		],
	);

	// List all linked URLs of a deployment
	app.get(
		"/:deploymentId/managed-urls",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::deployment::INFO,
				resource: closure_as_pinned_box!(|mut context| {
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
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(list_linked_urls)),
		],
	);

	// get all deployment metrics
	app.get(
		"/:deploymentId/metrics",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::deployment::INFO,
				resource: closure_as_pinned_box!(|mut context| {
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
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(get_deployment_metrics)),
		],
	);

	app.get(
		"/:deploymentId/build-logs",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::deployment::INFO,
				resource: closure_as_pinned_box!(|mut context| {
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
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(get_build_logs)),
		],
	);

	app.get(
		"/:deploymentId/events",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::deployment::INFO,
				resource: closure_as_pinned_box!(|mut context| {
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
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
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
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Listing deployments", request_id);

	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let user_token = context.get_token_data().status(500)?.clone();

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
	.filter(|deployment| {
		user_token.has_access_for_requested_action(
			&workspace_id,
			&deployment.id,
			permissions::workspace::infrastructure::deployment::INFO,
		)
	})
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

	context
		.success(ListDeploymentsResponse { deployments })
		.await?;
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
	_: NextHandler<EveContext, Error>,
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

	context
		.success(ListDeploymentHistoryResponse { deploys })
		.await?;
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
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Creating deployment", request_id);
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let ip_address = routes::get_request_ip_address(&context);

	let user_id = context.get_token_data().unwrap().user_id().clone();
	let login_id = context.get_token_data().unwrap().login_id().clone();

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

	service::update_cloudflare_kv_for_deployment(
		&id,
		deployment::Value::Created,
		&config,
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
				&now,
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

	context.success(CreateDeploymentResponse { id }).await?;
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
	_: NextHandler<EveContext, Error>,
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

	context
		.success(GetDeploymentInfoResponse {
			deployment,
			running_details,
		})
		.await?;
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
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Start deployment", request_id);

	let ip_address = routes::get_request_ip_address(&context);

	let user_id = context.get_token_data().unwrap().user_id().clone();

	let login_id = context.get_token_data().unwrap().login_id().clone();

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
		&now,
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

	context.success(StartDeploymentResponse {}).await?;
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
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();

	let ip_address = routes::get_request_ip_address(&context);

	let user_id = context.get_token_data().unwrap().user_id().clone();

	let login_id = context.get_token_data().unwrap().login_id().clone();

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

	context.success(StopDeploymentResponse {}).await?;
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
	_: NextHandler<EveContext, Error>,
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

	context.success(RevertDeploymentResponse {}).await?;
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
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let GetDeploymentLogsRequest {
		limit,
		end_time,
		start_time,
		..
	} = context
		.get_query_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let request_id = Uuid::new_v4();

	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();

	let config = context.get_state().config.clone();

	let end_time = end_time
		.map(|DateTime(end_time)| end_time)
		.unwrap_or_else(Utc::now);

	let start_time = start_time
		.map(|DateTime(end_time)| end_time)
		.unwrap_or_else(|| {
			// Loki query limit to 721h in time range, but it current loki is
			// not working as expected if query limit is 7 days or more,
			// so limitting it to 2 day
			end_time - Duration::days(2)
		});

	log::trace!("request_id: {} - Getting logs", request_id);
	let logs = service::get_deployment_container_logs(
		context.get_database_connection(),
		&deployment_id,
		&start_time,
		&end_time,
		limit.unwrap_or(100),
		&config,
		&request_id,
	)
	.await?;

	context.success(GetDeploymentLogsResponse { logs }).await?;
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
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();

	let DeleteDeploymentRequest {
		workspace_id: _,
		deployment_id: _,
		hard_delete,
	} = context
		.get_query_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let ip_address = routes::get_request_ip_address(&context);

	let user_id = context.get_token_data().unwrap().user_id().clone();

	let login_id = context.get_token_data().unwrap().login_id().clone();

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

		let volumes = db::get_all_deployment_volumes(
			context.get_database_connection(),
			&deployment.id,
		)
		.await?;

		for volume in volumes {
			db::stop_volume_usage_history(
				context.get_database_connection(),
				&volume.volume_id,
				&Utc::now(),
			)
			.await?;
		}
	}

	log::trace!("request_id: {} - Checking is any managed url is used by the deployment: {}", request_id, deployment_id);
	let managed_url = db::get_all_managed_urls_for_deployment(
		context.get_database_connection(),
		&deployment_id,
		&workspace_id,
	)
	.await?;

	if !managed_url.is_empty() {
		log::trace!(
			"deployment: {} - is using managed_url. Cannot delete it",
			deployment_id
		);
		return Error::as_result()
			.status(400)
			.body(error!(RESOURCE_IN_USE).to_string())?;
	}

	let region = db::get_region_by_id(
		context.get_database_connection(),
		&deployment.region,
	)
	.await?
	.status(500)?;

	let delete_k8s_resource = if region.is_patr_region() {
		true
	} else {
		hard_delete
	};

	log::trace!("request_id: {} - Deleting deployment", request_id);
	service::delete_deployment(
		context.get_database_connection(),
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

	let _ = service::get_internal_metrics(
		context.get_database_connection(),
		"A deployment has been deleted",
	)
	.await;

	// Commiting transaction so that even if the mailing function fails the
	// resource should be deleted
	context.commit_database_transaction().await?;

	service::resource_delete_action_email(
		context.get_database_connection(),
		&deployment.name,
		&deployment.workspace_id,
		&ResourceType::Deployment,
		&user_id,
	)
	.await?;

	context.success(DeleteDeploymentResponse {}).await?;
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
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	// workspace_id in UpdateDeploymentRequest struct parsed as null uuid(0..0),
	// hence taking the value here which will be same
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();

	log::trace!(
		"request_id: {} - Updating deployment with id: {}",
		request_id,
		deployment_id
	);
	let UpdateDeploymentRequest {
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
		volumes,
		..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let name = name.as_ref().map(|name| name.trim());

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
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	let config = context.get_state().config.clone();

	service::update_deployment(
		context.get_database_connection(),
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

	context.commit_database_transaction().await?;

	service::queue_check_and_update_deployment_status(
		&workspace_id,
		&deployment_id,
		&config,
		&request_id,
	)
	.await?;

	context.success(UpdateDeploymentResponse {}).await?;
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
	_: NextHandler<EveContext, Error>,
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

	context.success(ListLinkedURLsResponse { urls }).await?;
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
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();
	let query = context.get_query_as::<HashMap<String, String>>()?;

	let deployment = db::get_deployment_by_id(
		context.get_database_connection(),
		&deployment_id,
	)
	.await?
	.status(500)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let region = db::get_region_by_id(
		context.get_database_connection(),
		&deployment.region,
	)
	.await?
	.status(500)?;

	let tenant_id = if region.is_byoc_region() {
		deployment.workspace_id.as_str()
	} else {
		PATR_CLUSTER_TENANT_ID
	};

	log::trace!(
		"request_id: {} - Getting deployment metrics for deployment: {}",
		request_id,
		deployment_id
	);
	let start_time = Utc::now() -
		match query
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

	let step = query
		.get(request_keys::INTERVAL)
		.and_then(|value| value.parse::<Step>().ok())
		.unwrap_or(Step::TenMinutes);

	let config = context.get_state().config.clone();

	let deployment_metrics = service::get_deployment_metrics(
		tenant_id,
		&deployment_id,
		&config,
		&start_time,
		&Utc::now(),
		&step.to_string(),
		&request_id,
	)
	.await?;

	context
		.success(GetDeploymentMetricsResponse {
			metrics: deployment_metrics,
		})
		.await?;
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
	_: NextHandler<EveContext, Error>,
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
	context
		.success(GetDeploymentBuildLogsResponse { logs })
		.await?;
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
	_: NextHandler<EveContext, Error>,
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
	context
		.success(GetDeploymentEventsResponse { logs: build_events })
		.await?;
	Ok(context)
}
