use api_models::{
	models::prelude::*,
	utils::{DateTime, DecodedRequest, Uuid},
};
use axum::{extract::State, Extension, Router};
use chrono::{DateTime as ChronoDateTime, Utc};

use crate::{
	app::App,
	db,
	models::UserAuthenticationData,
	prelude::*,
	redis,
	service,
	utils::Error,
};

pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			create_api_token,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			list_api_tokens_for_user,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			list_permissions_for_api_token,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			regenerate_api_token,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			revoke_api_token,
		)
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			update_api_token,
		)
}

async fn create_api_token(
	mut connection: Connection,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: _,
		query: _,
		body:
			CreateApiTokenRequest {
				name,
				permissions,
				token_nbf,
				token_exp,
				allowed_ips,
			},
	}: DecodedRequest<CreateApiTokenRequest>,
) -> Result<CreateApiTokenResponse, Error> {
	let request_id = Uuid::new_v4();

	let user_id = token_data.user_id();

	let (id, token) = service::create_api_token_for_user(
		&mut connection,
		&user_id,
		&name,
		&permissions,
		&token_nbf.map(ChronoDateTime::<Utc>::from),
		&token_exp.map(ChronoDateTime::<Utc>::from),
		&allowed_ips,
		&request_id,
	)
	.await?;

	Ok(CreateApiTokenResponse { id, token })
}

async fn list_api_tokens_for_user(
	mut connection: Connection,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: _,
		query: _,
		body: _,
	}: DecodedRequest<ListApiTokensRequest>,
) -> Result<ListApiTokenResponse, Error> {
	let request_id = Uuid::new_v4();
	let user_id = token_data.user_id();

	log::trace!(
		"request_id: {} listing api_tokens for user: {}",
		request_id,
		user_id
	);
	let tokens = db::list_active_api_tokens_for_user(&mut connection, &user_id)
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

	Ok(ListApiTokenResponse { tokens })
}

async fn list_permissions_for_api_token(
	mut connection: Connection,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path,
		query: _,
		body: _,
	}: DecodedRequest<ListApiTokenPermissionsRequest>,
) -> Result<ListApiTokenPermissionsResponse, Error> {
	let request_id = Uuid::new_v4();
	let token_id = &path.token_id;
	let user_id = token_data.user_id();

	// Check if token exists
	db::get_active_user_api_token_by_id(&mut connection, &token_id)
		.await?
		.filter(|token| &token.user_id == user_id)
		.ok_or_else(|| ErrorType::NotFound)?;

	let old_permissions =
		service::get_permissions_for_user_api_token(&mut connection, &token_id)
			.await?;

	let new_permissions =
		service::get_revalidated_permissions_for_user_api_token(
			&mut connection,
			&token_id,
			&user_id,
		)
		.await?;

	if old_permissions != new_permissions {
		// Write the new config to the db
		db::remove_all_super_admin_permissions_for_api_token(
			&mut connection,
			&token_id,
		)
		.await?;
		db::remove_all_resource_type_permissions_for_api_token(
			&mut connection,
			&token_id,
		)
		.await?;
		db::remove_all_resource_permissions_for_api_token(
			&mut connection,
			&token_id,
		)
		.await?;

		for (workspace_id, permission) in &new_permissions {
			if permission.is_super_admin {
				db::add_super_admin_permission_for_api_token(
					&mut connection,
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
						&mut connection,
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
						&mut connection,
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

	Ok(ListApiTokenPermissionsResponse {
		permissions: new_permissions,
	})
}

async fn regenerate_api_token(
	mut connection: Connection,
	Extension(token_data): Extension<UserAuthenticationData>,
	State(mut app): State<App>,
	DecodedRequest {
		path,
		query: _,
		body: _,
	}: DecodedRequest<RegenerateApiTokenRequest>,
) -> Result<RegenerateApiTokenResponse, Error> {
	let token_id = &path.token_id;

	let token = Uuid::new_v4().to_string();
	let user_facing_token = format!("patrv1.{}.{}", token, token_id);
	let token_hash = service::hash(token.as_bytes())?;

	db::update_token_hash_for_user_api_token(
		&mut connection,
		&token_id,
		&token_hash,
	)
	.await?;
	redis::delete_user_api_token_data(&mut app.redis, &token_id).await?;

	Ok(RegenerateApiTokenResponse {
		token: user_facing_token,
	})
}

async fn revoke_api_token(
	mut connection: Connection,
	Extension(token_data): Extension<UserAuthenticationData>,
	State(mut app): State<App>,
	DecodedRequest {
		path,
		query: _,
		body: _,
	}: DecodedRequest<RevokeApiTokenRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	let user_id = token_data.user_id();

	let token_id = &path.token_id;

	db::get_active_user_api_token_by_id(&mut connection, &token_id)
		.await?
		.filter(|token| &token.user_id == user_id)
		.ok_or_else(|| ErrorType::NotFound)?;

	log::trace!(
		"request_id: {} with user_id: {} revoking api_token: {}",
		request_id,
		user_id,
		token_id
	);

	db::revoke_user_api_token(&mut connection, &token_id, &Utc::now()).await?;
	redis::delete_user_api_token_data(&mut app.redis, &token_id).await?;

	Ok(())
}

async fn update_api_token(
	mut connection: Connection,
	Extension(token_data): Extension<UserAuthenticationData>,
	State(mut app): State<App>,
	DecodedRequest {
		path,
		query: _,
		body:
			UpdateApiTokenRequest {
				token_id: _,
				name,
				permissions,
				token_nbf,
				token_exp,
				allowed_ips,
			},
	}: DecodedRequest<UpdateApiTokenRequest>,
) -> Result<(), Error> {
	let user_id = token_data.user_id();
	let token_id = &path.token_id;

	service::update_user_api_token(
		&mut connection,
		&mut app.redis,
		&token_id,
		&user_id,
		name.as_deref(),
		token_nbf.map(|DateTime(timestamp)| timestamp).as_ref(),
		token_exp.map(|DateTime(timestamp)| timestamp).as_ref(),
		allowed_ips.as_deref(),
		permissions.as_ref(),
	)
	.await?;

	Ok(())
}
