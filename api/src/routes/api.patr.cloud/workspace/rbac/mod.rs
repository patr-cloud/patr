use api_models::{
	models::workspace::rbac::{
		list_all_permissions::{ListAllPermissionsResponse, Permission},
		list_all_resource_types::{ListAllResourceTypesResponse, ResourceType},
		GetCurrentPermissionsResponse,
	},
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac,
	pin_fn,
	utils::{constants::request_keys, Error, EveContext, EveMiddleware},
};

mod role;
mod user;

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
	let mut sub_app = create_eve_app(app);

	sub_app.use_sub_app("/user", user::create_sub_app(app));
	sub_app.use_sub_app("/role", role::create_sub_app(app));

	sub_app.get(
		"/permission",
		[
			EveMiddleware::PlainTokenAuthenticator {
				is_api_token_allowed: true,
			},
			EveMiddleware::CustomFunction(pin_fn!(get_all_permissions)),
		],
	);
	sub_app.get(
		"/resource-type",
		[
			EveMiddleware::PlainTokenAuthenticator {
				is_api_token_allowed: true,
			},
			EveMiddleware::CustomFunction(pin_fn!(get_all_resource_types)),
		],
	);
	sub_app.get(
		"/current-permissions",
		[
			EveMiddleware::PlainTokenAuthenticator {
				is_api_token_allowed: true,
			},
			EveMiddleware::CustomFunction(pin_fn!(get_current_permissions)),
		],
	);

	sub_app
}

async fn get_all_permissions(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
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

	let permissions =
		db::get_all_permissions(context.get_database_connection())
			.await?
			.into_iter()
			.map(|permission| Permission {
				id: permission.id,
				name: permission.name,
				description: permission.description,
			})
			.collect();

	context
		.success(ListAllPermissionsResponse { permissions })
		.await?;
	Ok(context)
}

async fn get_all_resource_types(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
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

	let resource_types =
		db::get_all_resource_types(context.get_database_connection())
			.await?
			.into_iter()
			.map(|resource_type| ResourceType {
				id: resource_type.id,
				name: resource_type.name,
				description: resource_type.description,
			})
			.collect();

	context
		.success(ListAllResourceTypesResponse { resource_types })
		.await?;
	Ok(context)
}

async fn get_current_permissions(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let permissions = context
		.get_token_data()
		.unwrap()
		.workspace_permissions()
		.get(&workspace_id)
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?
		.clone();

	context
		.success(GetCurrentPermissionsResponse { permissions })
		.await?;
	Ok(context)
}
