use std::net::SocketAddr;

use api_models::{
	models::workspace::infrastructure::list_all_deployment_machine_type::{
		DeploymentMachineType,
		ListAllDeploymentMachineTypesResponse,
	},
	utils::Uuid,
};
use axum::{
	extract::{ConnectInfo, State},
	http::{Request, StatusCode},
	middleware::Next,
	response::Response,
	routing::get,
	Error,
	Router,
};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::{self, permissions},
	utils::{constants::request_keys, resource_token_authenticator},
};

mod deployment;
mod managed_database;
mod managed_url;
mod static_site;

pub fn create_sub_route(app: &App) -> Router {
	let mut router = Router::new()
		.merge(
			Router::new()
				.nest("/deployment", deployment::create_sub_route(app)),
		)
		.merge(
			Router::new().nest(
				"/managed-database",
				managed_database::create_sub_route(app),
			),
		)
		.merge(
			Router::new()
				.nest("/managed-url", managed_url::create_sub_route(app)),
		)
		.merge(
			Router::new()
				.nest("/static-site", static_site::create_sub_route(app)),
		)
		//  Route uses plainTokenAuthenticator
		.merge(
			Router::new()
				.route("/machine-type", get(get_all_deployment_machine_types)),
		);

	router
}

async fn list_deployment_permission<B>(
	State(app): State<App>,
	ip_addr: ConnectInfo<SocketAddr>,
	Path(workspace_id): Path<Uuid>,
	request: Request<B>,
	next: Next<B>,
) -> Result<Response, Error> {
	let is_api_token_allowed = true;
	// TODO - Figure out how to get db connection
	let resource = if let Some(resource) =
		db::get_resource_by_id(connection, workspace_id).await?
	{
		resource
	} else {
		Err(Error::new(StatusCode::NOT_FOUND.into()))
	};
	let permission = permissions::workspace::infrastructure::deployment::LIST;

	// TODO - call PlainTokenAuthenticator with is_api_token_allowed
	let allowed = resource_token_authenticator(
		&app,
		&request,
		&ip_addr,
		is_api_token_allowed,
		&resource,
		permission,
	)
	.await?;

	if allowed {
		Ok(next.run(request).await)
	} else {
		Err(Error::new(StatusCode::UNAUTHORIZED.into()))
	}

	// TODO - remove this after error is fixed
	Ok(next.run(request).await)
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

// pub async fn check_create_deployment_permission () -> {

// }
