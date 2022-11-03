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
use chrono::{DateTime as ChronoDateTime, Utc};
use eve_rs::{App as EveApp, AsError, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	pin_fn,
	redis,
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

	app.post(
		"/api-token",
		[
			EveMiddleware::PlainTokenAuthenticator {
				is_api_token_allowed: false,
			},
			EveMiddleware::CustomFunction(pin_fn!(create_api_token)),
		],
	);
	app.get(
		"/api-token",
		[
			EveMiddleware::PlainTokenAuthenticator {
				is_api_token_allowed: false,
			},
			EveMiddleware::CustomFunction(pin_fn!(list_api_tokens_for_user)),
		],
	);
	app.get(
		"/api-token/:tokenId/permission",
		[
			EveMiddleware::PlainTokenAuthenticator {
				is_api_token_allowed: false,
			},
			EveMiddleware::CustomFunction(pin_fn!(
				list_permissions_for_api_token
			)),
		],
	);
	app.post(
		"/api-token/:tokenId/regenerate",
		[
			EveMiddleware::PlainTokenAuthenticator {
				is_api_token_allowed: false,
			},
			EveMiddleware::CustomFunction(pin_fn!(regenerate_api_token)),
		],
	);
	app.post(
		"/api-token/:tokenId/revoke",
		[
			EveMiddleware::PlainTokenAuthenticator {
				is_api_token_allowed: false,
			},
			EveMiddleware::CustomFunction(pin_fn!(revoke_api_token)),
		],
	);
	app.patch(
		"/api-token/:tokenId",
		[
			EveMiddleware::PlainTokenAuthenticator {
				is_api_token_allowed: false,
			},
			EveMiddleware::CustomFunction(pin_fn!(update_api_token)),
		],
	);

	app
}

async fn create_api_token(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
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
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
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
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
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
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
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

async fn revoke_api_token(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
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

async fn update_api_token(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
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
