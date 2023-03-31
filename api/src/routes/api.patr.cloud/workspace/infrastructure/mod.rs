use api_models::{
	models::workspace::infrastructure::list_all_deployment_machine_type::{
		DeploymentMachineType,
		ListAllDeploymentMachineTypesResponse,
	},
	utils::Uuid,
};
use axum::{extract::State, routing::get, Router};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac,
	utils::{constants::request_keys, Error},
};

mod deployment;
mod managed_database;
mod managed_url;
mod static_site;

pub fn create_sub_route(app: &App) -> Router {
	let mut sub_app = create_eve_app(app);
	let mut router = Router::new();

	router.nest("/deployment", deployment::create_sub_route(app));
	router.nest("/managed-database", managed_database::create_sub_route(app));
	router.nest("/managed-url", managed_url::create_sub_route(app));
	router.nest("/static-site", static_site::create_sub_route(app));

	//  Route uses plainTokenAuthenticator
	router.route("/machine-type", get(get_all_deployment_machine_types));

	sub_app
}

async fn get_all_deployment_machine_types(
	State(app): State<App>,
) -> Result<EveContext, Error> {
	let workspace_id = context
		.get_param(request_keys::WORKSPACE_ID)
		.and_then(|workspace_id| Uuid::parse_str(workspace_id).ok())
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let access_token_data = context.get_token_data().unwrap();
	let god_user_id = rbac::GOD_USER_ID.get().unwrap();

	if !access_token_data
		.workspace_permissions()
		.contains_key(&workspace_id) &&
		access_token_data.user_id() != god_user_id
	{
		Error::as_result()
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	}

	let machine_types =
		db::get_all_deployment_machine_types(context.get_database_connection())
			.await?
			.into_iter()
			.map(|machine_type| DeploymentMachineType {
				id: machine_type.id,
				cpu_count: machine_type.cpu_count,
				memory_count: machine_type.memory_count,
			})
			.collect();

	context.success(ListAllDeploymentMachineTypesResponse { machine_types });
	Ok(context)
}
