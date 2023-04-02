use api_models::{
	models::user::{
		CreateApiTokenRequest,
		CreateApiTokenResponse,
		ListApiTokenPermissionsResponse,
		ListApiTokenResponse,
		RegenerateApiTokenResponse,
		RevokeApiTokenResponse,
		UpdateApiTokenRequest,
		UpdateApiTokenResponse,
		UserApiToken,
	},
	utils::{DateTime, Uuid},
};
use axum::{
	extract::State,
	middleware,
	routing::{get, patch, post},
	Router,
};
use chrono::{DateTime as ChronoDateTime, Utc};

use crate::{
	app::App,
	db,
	error,
	redis,
	routes::plain_token_authenticator_without_api_token,
	service,
	utils::{constants::request_keys, Error},
};

pub fn create_sub_route(app: &App) -> Router<App> {
	// All routes have plainTokenAuthenticator
	let router = Router::new()
		.merge(
			Router::new()
				.route("/api-token", post(create_api_token))
				.route_layer(middleware::from_fn_with_state(
					app.clone(),
					plain_token_authenticator_without_api_token,
				)),
		)
		.merge(
			Router::new()
				.route("/api-token", get(list_api_tokens_for_user))
				.route_layer(middleware::from_fn_with_state(
					app.clone(),
					plain_token_authenticator_without_api_token,
				)),
		)
		.merge(
			Router::new()
				.route(
					"/api-token/:tokenId/permission",
					get(list_permissions_for_api_token),
				)
				.route_layer(middleware::from_fn_with_state(
					app.clone(),
					plain_token_authenticator_without_api_token,
				)),
		)
		.merge(
			Router::new()
				.route(
					"/api-token/:tokenId/regenerate",
					post(regenerate_api_token),
				)
				.route_layer(middleware::from_fn_with_state(
					app.clone(),
					plain_token_authenticator_without_api_token,
				)),
		)
		.merge(
			Router::new()
				.route("/api-token/:tokenId/revoke", post(revoke_api_token))
				.route_layer(middleware::from_fn_with_state(
					app.clone(),
					plain_token_authenticator_without_api_token,
				)),
		)
		.merge(
			Router::new()
				.route("/api-token/:tokenId", patch(update_api_token))
				.route_layer(middleware::from_fn_with_state(
					app.clone(),
					plain_token_authenticator_without_api_token,
				)),
		);

	router
}

async fn create_api_token(State(app): State<App>) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let user_id = context.get_token_data().unwrap().user_id().clone();

	let CreateApiTokenRequest {
		name,
		permissions,
		token_nbf,
		token_exp,
		allowed_ips,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let (id, token) = service::create_api_token_for_user(
		context.get_database_connection(),
		&user_id,
		&name,
		&permissions,
		&token_nbf.map(ChronoDateTime::<Utc>::from),
		&token_exp.map(ChronoDateTime::<Utc>::from),
		&allowed_ips,
		&request_id,
	)
	.await?;

	context.success(CreateApiTokenResponse { id, token });
	Ok(context)
}

async fn list_api_tokens_for_user(
	State(app): State<App>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let user_id = context.get_token_data().unwrap().user_id().clone();

	log::trace!(
		"request_id: {} listing api_tokens for user: {}",
		request_id,
		user_id
	);
	let tokens = db::list_active_api_tokens_for_user(
		context.get_database_connection(),
		&user_id,
	)
	.await?
	.into_iter()
	.map(|token| UserApiToken {
		id: token.token_id,
		name: token.name,
		token_nbf: token.token_nbf.map(DateTime),
		token_exp: token.token_exp.map(DateTime),
		allowed_ips: token.allowed_ips,
		created: DateTime(token.created),
	})
	.collect::<Vec<_>>();

	context.success(ListApiTokenResponse { tokens });
	Ok(context)
}

async fn list_permissions_for_api_token(
	State(app): State<App>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let token_id = context.get_param(request_keys::TOKEN_ID).unwrap();
	let token_id = Uuid::parse_str(token_id)
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	let user_id = context.get_token_data().unwrap().user_id().clone();

	// Check if token exists
	db::get_active_user_api_token_by_id(
		context.get_database_connection(),
		&token_id,
	)
	.await?
	.filter(|token| token.user_id == user_id)
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let old_permissions = service::get_permissions_for_user_api_token(
		context.get_database_connection(),
		&token_id,
	)
	.await?;

	let new_permissions =
		service::get_revalidated_permissions_for_user_api_token(
			context.get_database_connection(),
			&token_id,
			&user_id,
		)
		.await?;

	if old_permissions != new_permissions {
		// Write the new config to the db
		db::remove_all_super_admin_permissions_for_api_token(
			context.get_database_connection(),
			&token_id,
		)
		.await?;
		db::remove_all_resource_type_permissions_for_api_token(
			context.get_database_connection(),
			&token_id,
		)
		.await?;
		db::remove_all_resource_permissions_for_api_token(
			context.get_database_connection(),
			&token_id,
		)
		.await?;

		for (workspace_id, permission) in &new_permissions {
			if permission.is_super_admin {
				db::add_super_admin_permission_for_api_token(
					context.get_database_connection(),
					&token_id,
					workspace_id,
					&user_id,
				)
				.await?;
			}

			for (resource_type_id, permissions) in
				&permission.resource_type_permissions
			{
				for permission_id in permissions {
					db::add_resource_type_permission_for_api_token(
						context.get_database_connection(),
						&token_id,
						workspace_id,
						resource_type_id,
						permission_id,
					)
					.await?;
				}
			}

			for (resource_id, permissions) in &permission.resource_permissions {
				for permission_id in permissions {
					db::add_resource_permission_for_api_token(
						context.get_database_connection(),
						&token_id,
						workspace_id,
						resource_id,
						permission_id,
					)
					.await?;
				}
			}
		}
	}

	log::trace!(
		"request_id: {} listing permissions for api_token: {}",
		request_id,
		token_id
	);

	context.success(ListApiTokenPermissionsResponse {
		permissions: new_permissions,
	});
	Ok(context)
}

async fn regenerate_api_token(
	State(app): State<App>,
) -> Result<EveContext, Error> {
	let token_id =
		Uuid::parse_str(context.get_param(request_keys::TOKEN_ID).unwrap())?;

	let token = Uuid::new_v4().to_string();
	let user_facing_token = format!("patrv1.{}.{}", token, token_id);
	let token_hash = service::hash(token.as_bytes())?;

	db::update_token_hash_for_user_api_token(
		context.get_database_connection(),
		&token_id,
		&token_hash,
	)
	.await?;
	redis::delete_user_api_token_data(
		context.get_redis_connection(),
		&token_id,
	)
	.await?;

	context.success(RegenerateApiTokenResponse {
		token: user_facing_token,
	});
	Ok(context)
}

async fn revoke_api_token(State(app): State<App>) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let user_id = context.get_token_data().unwrap().user_id().clone();

	let token_id = context.get_param(request_keys::TOKEN_ID).unwrap();
	let token_id = Uuid::parse_str(token_id)
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	db::get_active_user_api_token_by_id(
		context.get_database_connection(),
		&token_id,
	)
	.await?
	.filter(|token| token.user_id == user_id)
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	log::trace!(
		"request_id: {} with user_id: {} revoking api_token: {}",
		request_id,
		user_id,
		token_id
	);

	db::revoke_user_api_token(
		context.get_database_connection(),
		&token_id,
		&Utc::now(),
	)
	.await?;
	redis::delete_user_api_token_data(
		context.get_redis_connection(),
		&token_id,
	)
	.await?;

	context.success(RevokeApiTokenResponse {});
	Ok(context)
}

async fn update_api_token(State(app): State<App>) -> Result<EveContext, Error> {
	let user_id = context.get_token_data().unwrap().user_id().clone();
	let token_id =
		Uuid::parse_str(context.get_param(request_keys::TOKEN_ID).unwrap())?;

	let UpdateApiTokenRequest {
		token_id: _,
		name,
		permissions,
		token_nbf,
		token_exp,
		allowed_ips,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let mut redis_connection = context.get_redis_connection().clone();

	service::update_user_api_token(
		context.get_database_connection(),
		&mut redis_connection,
		&token_id,
		&user_id,
		name.as_deref(),
		token_nbf.map(|DateTime(timestamp)| timestamp).as_ref(),
		token_exp.map(|DateTime(timestamp)| timestamp).as_ref(),
		allowed_ips.as_deref(),
		permissions.as_ref(),
	)
	.await?;

	context.success(UpdateApiTokenResponse {});
	Ok(context)
}
