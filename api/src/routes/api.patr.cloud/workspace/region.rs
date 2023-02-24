use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::region::{
		AddRegionToWorkspaceData,
		AddRegionToWorkspaceRequest,
		AddRegionToWorkspaceResponse,
		DeleteRegionFromWorkspaceRequest,
		DeleteRegionFromWorkspaceResponse,
		GetRegionInfoResponse,
		InfrastructureCloudProvider,
		ListRegionsForWorkspaceResponse,
		Region,
		RegionStatus,
		RegionType,
	},
	utils::Uuid,
};
use chrono::Utc;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::{self, permissions},
	pin_fn,
	routes,
	service,
	utils::{
		constants::request_keys,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut app = create_eve_app(app);

	// List all regions
	app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::region::LIST,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(list_regions)),
		],
	);

	// Get region
	app.get(
		"/:regionId",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::region::INFO,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let region_id =
						context.get_param(request_keys::REGION_ID).unwrap();
					let region_id = Uuid::parse_str(region_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&region_id,
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
			},
			EveMiddleware::CustomFunction(pin_fn!(get_region)),
		],
	);

	// Add a new region
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::region::ADD,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(add_region)),
		],
	);

	// Delete a new region
	app.delete(
		"/:regionId",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::region::DELETE,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let region_id =
						context.get_param(request_keys::REGION_ID).unwrap();
					let region_id = Uuid::parse_str(region_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&region_id,
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
			},
			EveMiddleware::CustomFunction(pin_fn!(delete_region)),
		],
	);

	app
}

async fn list_regions(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - Listing all regions", request_id);
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let regions = db::get_all_deployment_regions_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.map(|region| Region {
		r#type: if region.is_byoc_region() {
			RegionType::BYOC
		} else {
			RegionType::PatrOwned
		},
		id: region.id,
		name: region.name,
		cloud_provider: region.cloud_provider,
		status: region.status,
	})
	.collect();

	log::trace!("request_id: {} - Returning regions", request_id);
	context.success(ListRegionsForWorkspaceResponse { regions });
	Ok(context)
}

async fn get_region(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - Listing all regions", request_id);

	let region_id =
		Uuid::parse_str(context.get_param(request_keys::REGION_ID).unwrap())
			.unwrap();

	let region =
		db::get_region_by_id(context.get_database_connection(), &region_id)
			.await?
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	if region.workspace_id.is_some() && region.disconnected_at.is_none() {
		return Error::as_result()
			.status(500)
			.body(error!(REGION_NOT_CONNECTED).to_string())?;
	}

	let region_response = Region {
		r#type: if region.is_byoc_region() {
			RegionType::BYOC
		} else {
			RegionType::PatrOwned
		},
		id: region.id,
		name: region.name,
		cloud_provider: region.cloud_provider,
		status: region.status,
	};

	log::trace!("request_id: {} - Returning region", request_id);
	context.success(GetRegionInfoResponse {
		region: region_response,
		disconnected_at: region.disconnected_at.map(Into::into),
		message_log: region.message_log,
	});
	Ok(context)
}

async fn add_region(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let AddRegionToWorkspaceRequest {
		data,
		name,
		workspace_id: _,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();

	log::trace!(
		"{} - Adding new region to workspace {}",
		request_id,
		workspace_id,
	);

	let region_id =
		db::generate_new_resource_id(context.get_database_connection()).await?;

	db::create_resource(
		context.get_database_connection(),
		&region_id,
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(crate::models::rbac::resource_types::DEPLOYMENT_REGION)
			.unwrap(),
		&workspace_id,
		&Utc::now(),
	)
	.await?;

	match data {
		AddRegionToWorkspaceData::Digitalocean {
			region,
			api_token,
			cluster_name,
			min_nodes,
			max_nodes,
			auto_scale,
			node_name,
			node_size_slug,
		} => {
			log::trace!(
				"request_id: {} creating digital ocean k8s cluster in db",
				request_id
			);
			db::add_deployment_region_to_workspace(
				context.get_database_connection(),
				&region_id,
				&name,
				&InfrastructureCloudProvider::Digitalocean,
				&workspace_id,
			)
			.await?;

			context.commit_database_transaction().await?;

			let cluster_id = service::create_do_k8s_cluster(
				&region.to_string(),
				&api_token,
				&cluster_name,
				min_nodes,
				max_nodes,
				auto_scale,
				&node_name,
				&node_size_slug,
				&request_id,
			)
			.await?;

			log::trace!(
				"reqeust_id: {} successfully got k8s ID: {}",
				request_id,
				cluster_id
			);

			service::queue_get_kube_config_for_do_cluster(
				&api_token,
				&cluster_id,
				&region_id,
				&config,
				&request_id,
			)
			.await?;

			// TODO - send the appropriate email for success/failure of the
			// region
		}
		AddRegionToWorkspaceData::KubeConfig { config_file } => {
			db::add_deployment_region_to_workspace(
				context.get_database_connection(),
				&region_id,
				&name,
				&InfrastructureCloudProvider::Other,
				&workspace_id,
			)
			.await?;

			context.commit_database_transaction().await?;

			service::queue_setup_kubernetes_cluster(
				&region_id,
				config_file,
				&config,
				&request_id,
			)
			.await?;
		}
	}

	log::trace!(
		"request_id: {} - Successfully added region to workspace",
		request_id
	);
	context.success(AddRegionToWorkspaceResponse { region_id });
	Ok(context)
}

async fn delete_region(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let ip_address = routes::get_request_ip_address(&context);
	let user_id = context.get_token_data().unwrap().user_id().clone();
	let login_id = context.get_token_data().unwrap().login_id().clone();

	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let region_id =
		Uuid::parse_str(context.get_param(request_keys::REGION_ID).unwrap())
			.unwrap();

	let DeleteRegionFromWorkspaceRequest {
		workspace_id: _,
		region_id: _,
		hard_delete,
	} = context
		.get_query_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	log::trace!(
		"{} - requested to delete region: {} from workspace {}",
		request_id,
		region_id,
		workspace_id,
	);

	let region =
		db::get_region_by_id(context.get_database_connection(), &region_id)
			.await?
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	// todo: how to delete the region if it got stuck in creating state
	if region.status != RegionStatus::Active {
		return Error::as_result()
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	}

	let config = context.get_state().config.clone();

	log::trace!(
		"request_id: {} is getting all the deployment for region: {}",
		request_id,
		region_id
	);

	let deployments = db::get_deployments_by_region_id(
		context.get_database_connection(),
		&workspace_id,
		&region_id,
	)
	.await?;

	for deployment in &deployments {
		service::delete_deployment(
			context.get_database_connection(),
			&deployment.workspace_id,
			&deployment.id,
			&region_id,
			Some(&user_id),
			Some(&login_id),
			&ip_address,
			false,
			hard_delete,
			&config,
			&request_id,
		)
		.await?
	}

	if hard_delete {
		service::queue_delete_kubernetes_cluster(
			&region_id,
			&workspace_id,
			region.config_file.unwrap_or_default(),
			&config,
			&request_id,
		)
		.await?;
	}

	db::delete_region(
		context.get_database_connection(),
		&region_id,
		&Utc::now(),
	)
	.await?;

	// TODO send emails about the action
	context.success(DeleteRegionFromWorkspaceResponse {});
	Ok(context)
}
