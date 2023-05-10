use api_models::{models::prelude::*, utils::DtoRequestExt};
use axum::{extract::State, Extension, Router};

use crate::{
	app::App,
	db,
	models::{rbac, UserAuthenticationData},
	prelude::*,
	utils::Error,
};

mod deployment;
mod managed_database;
mod managed_url;
mod static_site;

pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.mount_protected_dto(
			PlainTokenAuthenticator::new(),
			app.clone(),
			get_all_deployment_machine_types,
		)
		.merge(deployment::create_sub_app(app))
		.merge(managed_url::create_sub_app(app))
		.merge(static_site::create_sub_app(app))
		.merge(managed_database::create_sub_app(app))
}

async fn get_all_deployment_machine_types(
	mut connection: Connection,
	State(app): State<App>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: ListAllDeploymentMachineTypesPath { workspace_id },
		query: (),
		body: ListAllDeploymentMachineTypesRequest { workspace_name },
	}: DecodedRequest<ListAllDeploymentMachineTypesRequest>,
) -> Result<CreateNewWorkspaceResponse, Error> {
	let god_user_id = rbac::GOD_USER_ID.get().unwrap();

	if !token_data
		.workspace_permissions()
		.contains_key(&workspace_id) &&
		token_data.user_id() != god_user_id
	{
		return Err(ErrorType::NotFound);
	}

	let machine_types = db::get_all_deployment_machine_types(&mut connection)
		.await?
		.into_iter()
		.map(|machine_type| DeploymentMachineType {
			id: machine_type.id,
			cpu_count: machine_type.cpu_count,
			memory_count: machine_type.memory_count,
		})
		.collect();

	Ok(ListAllDeploymentMachineTypesResponse { machine_types })
}
