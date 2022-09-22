use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::{
		region::{
			AddRegionToWorkspaceData,
			AddRegionToWorkspaceRequest,
			AddRegionToWorkspaceResponse,
			InfrastructureCloudProvider,
			ListRegionsForWorkspaceResponse,
			Region,
		},
		secret::DeleteSecretResponse,
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
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::region::LIST,
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
			EveMiddleware::CustomFunction(pin_fn!(list_regions)),
		],
	);

	// Add a new region
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::region::ADD,
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
			EveMiddleware::CustomFunction(pin_fn!(add_region)),
		],
	);

	// remove a region
	app.delete(
		"/:regionId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::region::REMOVE,
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
			EveMiddleware::CustomFunction(pin_fn!(remove_region)),
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
		id: region.id,
		name: region.name,
		cloud_provider: region.cloud_provider,
		ready: region.ready,
		default: region.workspace_id.is_none(),
		message_log: region.message_log,
	})
	.collect();

	log::trace!("request_id: {} - Returning regions", request_id);
	context.success(ListRegionsForWorkspaceResponse { regions });
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
		&format!("Region: {}", name),
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
			cloud_provider: _,
			region: _,
			api_token: _,
		} => {
			return Err(Error::empty()
				.body("Currently digital ocean api is not supported"))
		}
		AddRegionToWorkspaceData::KubernetesCluster {
			certificate_authority_data,
			cluster_url,
			auth_username,
			auth_token,
		} => {
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
				&cluster_url,
				&certificate_authority_data,
				&auth_username,
				&auth_token,
				&config,
				&request_id,
			)
			.await?;
		}
	}

	log::trace!("request_id: {} - Returning new secret", request_id);
	context.success(AddRegionToWorkspaceResponse { region_id });
	Ok(context)
}

async fn remove_region(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let secret_id =
		Uuid::parse_str(context.get_param(request_keys::SECRET_ID).unwrap())
			.unwrap();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let config = context.get_state().config.clone();

	log::trace!("request_id: {} - Deleting secret {}", request_id, secret_id);
	service::delete_secret_in_workspace(
		context.get_database_connection(),
		&workspace_id,
		&secret_id,
		&config,
		&request_id,
	)
	.await?;

	context.success(DeleteSecretResponse {});
	Ok(context)
}
