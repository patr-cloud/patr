use api_models::{models::prelude::*, utils::DecodedRequest, ErrorType};
use axum::{extract::State, Extension, Router};

use crate::{
	app::App,
	db,
	models::{rbac, UserAuthenticationData},
	prelude::*,
	utils::Error,
};

mod role;
mod user;

pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.mount_protected_dto(
			PlainTokenAuthenticator::new(),
			app.clone(),
			get_all_permissions,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new(),
			app.clone(),
			get_all_resource_types,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new(),
			app.clone(),
			get_current_permissions,
		)
		.merge(user::create_sub_app(app))
		.merge(role::create_sub_app(app))
}

async fn get_all_permissions(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(access_token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: ListAllPermissionsPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<ListAllPermissionsRequest>,
) -> Result<ListAllPermissionsResponse, Error> {
	let god_user_id = rbac::GOD_USER_ID.get().unwrap();

	if !access_token_data
		.workspace_permissions()
		.contains_key(&workspace_id) &&
		access_token_data.user_id() != god_user_id
	{
		return Err(ErrorType::NotFound);
	}

	let permissions = db::get_all_permissions(&mut connection)
		.await?
		.into_iter()
		.map(|permission| Permission {
			id: permission.id,
			name: permission.name,
			description: permission.description,
		})
		.collect();

	Ok(ListAllPermissionsResponse { permissions })
}

async fn get_all_resource_types(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(access_token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: ListAllResourceTypesPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<ListAllResourceTypesRequest>,
) -> Result<ListAllResourceTypesResponse, Error> {
	let god_user_id = rbac::GOD_USER_ID.get().unwrap();

	if !access_token_data
		.workspace_permissions()
		.contains_key(&workspace_id) &&
		access_token_data.user_id() != god_user_id
	{
		return Err(ErrorType::NotFound);
	}

	let resource_types = db::get_all_resource_types(&mut connection)
		.await?
		.into_iter()
		.map(|resource_type| ResourceType {
			id: resource_type.id,
			name: resource_type.name,
			description: resource_type.description,
		})
		.collect();

	Ok(ListAllResourceTypesResponse { resource_types })
}

async fn get_current_permissions(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(access_token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: GetCurrentPermissionsPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<GetCurrentPermissionsRequest>,
) -> Result<GetCurrentPermissionsResponse, Error> {
	let permissions = access_token_data
		.workspace_permissions()
		.get(&workspace_id)
		.ok_or_else(|| ErrorType::NotFound)?
		.clone();

	Ok(GetCurrentPermissionsResponse { permissions })
}
