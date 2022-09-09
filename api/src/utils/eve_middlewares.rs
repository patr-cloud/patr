use std::{collections::HashMap, future::Future, pin::Pin};

use api_models::utils::Uuid;
use async_trait::async_trait;
use chrono::Utc;
use eve_rs::{
	default_middlewares::{
		compression::CompressionHandler,
		cookie_parser::parser as cookie_parser,
		json::parser as json_parser,
		static_file_server::StaticFileServer,
		url_encoded::parser as url_encoded_parser,
	},
	App as EveApp,
	AsError,
	Context,
	Middleware,
	NextHandler,
};
use redis::aio::MultiplexedConnection as RedisConnection;

use crate::{
	app::App,
	db::{self, Resource},
	error,
	models::{
		rbac::{self, WorkspacePermissions, GOD_USER_ID},
		AccessTokenData,
		ApiTokenData,
	},
	redis::is_access_token_revoked,
	utils::{Error, ErrorData, EveContext},
};

pub type MiddlewareHandlerFunction =
	fn(
		EveContext,
		NextHandler<EveContext, ErrorData>,
	) -> Pin<Box<dyn Future<Output = Result<EveContext, Error>> + Send>>;

pub type ResourceRequiredFunction = fn(
	EveContext,
) -> Pin<
	Box<
		dyn Future<Output = Result<(EveContext, Option<Resource>), Error>>
			+ Send,
	>,
>;

#[allow(dead_code)]
#[derive(Clone)]
pub enum EveMiddleware {
	Compression(u32),
	JsonParser,
	UrlEncodedParser,
	CookieParser,
	StaticHandler(StaticFileServer),
	PlainTokenAuthenticator,
	ResourceTokenAuthenticator(&'static str, ResourceRequiredFunction), /* (permission, resource_required) */
	CustomFunction(MiddlewareHandlerFunction),
	DomainRouter(
		String,
		Box<EveApp<EveContext, EveMiddleware, App, ErrorData>>,
	),
	BlockApiToken,
}

#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum TokenData {
	AccessTokenData(AccessTokenData),
	ApiTokenData(ApiTokenData),
}

#[async_trait]
impl Middleware<EveContext, ErrorData> for EveMiddleware {
	async fn run_middleware(
		&self,
		mut context: EveContext,
		next: NextHandler<EveContext, ErrorData>,
	) -> Result<EveContext, Error> {
		match self {
			EveMiddleware::Compression(compression_level) => {
				let mut compressor =
					CompressionHandler::create(*compression_level);

				context = next(context).await?;
				compressor.compress(&mut context);

				Ok(context)
			}
			EveMiddleware::JsonParser => {
				if let Some(value) = json_parser(&context)? {
					context.set_body_object(value);
				}

				next(context).await
			}
			EveMiddleware::UrlEncodedParser => {
				if let Some(value) = url_encoded_parser(&context)? {
					context.set_body_object(value);
				}

				next(context).await
			}
			EveMiddleware::CookieParser => {
				cookie_parser(&mut context);
				next(context).await
			}
			EveMiddleware::StaticHandler(static_file_server) => {
				static_file_server.run_middleware(context, next).await
			}
			EveMiddleware::PlainTokenAuthenticator => {
				let token_data = decode_access_token(&mut context).await?;

				match token_data {
					TokenData::AccessTokenData(token_data) => {
						validate_access_token(
							context.get_redis_connection(),
							&token_data,
						)
						.await?;
						context.set_token_data(TokenData::AccessTokenData(
							token_data,
						));
					}
					TokenData::ApiTokenData(token_data) => {
						validate_api_token(&token_data).await?;
						context.set_token_data(TokenData::ApiTokenData(
							token_data,
						));
					}
				}
				next(context).await
			}
			EveMiddleware::ResourceTokenAuthenticator(
				permission_required,
				resource_in_question,
			) => {
				let token_data = decode_access_token(&mut context).await?;

				let (mut context, resource) =
					resource_in_question(context).await?;
				let resource = if let Some(resource) = resource {
					resource
				} else {
					return Ok(context);
				};

				match token_data {
					TokenData::AccessTokenData(token_data) => {
						validate_access_token(
							context.get_redis_connection(),
							&token_data,
						)
						.await?;

						context.set_token_data(TokenData::AccessTokenData(
							token_data.clone(),
						));
						let allowed = has_permission_to_access_resource(
							&resource,
							permission_required,
							TokenData::AccessTokenData(token_data.clone()),
						);

						if allowed {
							context.set_token_data(TokenData::AccessTokenData(
								token_data.clone(),
							));
							next(context).await
						} else {
							context.status(401).json(error!(UNAUTHORIZED));
							return Ok(context);
						}
					}
					TokenData::ApiTokenData(token_data) => {
						validate_api_token(&token_data).await?;

						context.set_token_data(TokenData::ApiTokenData(
							token_data.clone(),
						));
						let allowed = has_permission_to_access_resource(
							&resource,
							permission_required,
							TokenData::ApiTokenData(token_data.clone()),
						);

						if allowed {
							context.set_token_data(TokenData::ApiTokenData(
								token_data.clone(),
							));
							return next(context).await;
						} else {
							context.status(401).json(error!(UNAUTHORIZED));
							return Ok(context);
						}
					}
				}
			}

			EveMiddleware::CustomFunction(function) => {
				function(context, next).await
			}
			EveMiddleware::DomainRouter(domain, app) => {
				let localhost =
					format!("localhost:{}", app.get_state().config.port);
				if &context.get_host() == domain ||
					context.get_host() == localhost
				{
					app.resolve(context).await
				} else {
					next(context).await
				}
			}
			EveMiddleware::BlockApiToken => {
				let authorization =
					if let Some(header) = context.get_header("Authorization") {
						header
					} else {
						log::trace!("Authorization header not found");
						return Error::as_result()
							.status(401)
							.body(error!(UNAUTHORIZED).to_string())?;
					};

				let is_api_token = Uuid::parse_str(&authorization).is_ok();
				if !is_api_token {
					next(context).await
				} else {
					return Error::as_result()
						.status(403)
						.body(error!(UNPRIVILEGED).to_string());
				}
			}
		}
	}
}

async fn decode_access_token(
	context: &mut EveContext,
) -> Result<TokenData, Error> {
	let authorization = context
		.get_header("Authorization")
		.expect("Authorization header not found");
	let is_api_token = Uuid::parse_str(&authorization).is_ok();
	if is_api_token {
		let token = Uuid::parse_str(&authorization).unwrap();
		let api_token =
			db::get_api_token_by_id(context.get_database_connection(), &token)
				.await?;

		if let Some(api_token) = api_token {
			let is_token_valid = if let Some(expiry) = &api_token.token_expiry {
				let token_expiry = expiry.timestamp_millis();
				let current_time = Utc::now().timestamp_millis();
				token_expiry < current_time
			} else {
				true
			};
			if !is_token_valid {
				log::warn!("Token: {} is invalid", token);
				return Error::as_result()
					.status(401)
					.body(error!(UNAUTHORIZED).to_string())?;
			}
			let api_token_permissions = db::list_permissions_for_api_token(
				context.get_database_connection(),
				&token,
			)
			.await?;

			let workspace_id = db::get_workspace_id_for_api_token(
				context.get_database_connection(),
				&token,
			)
			.await?;

			let super_admin_token = db::get_super_admin_api_token(
				context.get_database_connection(),
				&token,
			)
			.await?;

			if workspace_id.is_none() {
				log::warn!("Cannot get workspace_id for api_token:{}", token);
				return Error::as_result()
					.status(401)
					.body(error!(UNAUTHORIZED).to_string())?;
			}

			let is_super_admin = if let Some(admin_token) = super_admin_token {
				!admin_token.super_admin_id.is_nil()
			} else {
				false
			};

			let api_token_data = ApiTokenData {
				exp: api_token.token_expiry,
				workspaces: HashMap::from([(
					workspace_id.unwrap(),
					WorkspacePermissions {
						is_super_admin,
						resources: api_token_permissions.resource_permissions,
						resource_types: api_token_permissions
							.resource_type_permissions,
					},
				)]),
				user_id: api_token.user_id,
			};
			Ok(TokenData::ApiTokenData(api_token_data))
		} else {
			log::warn!("Invalid api token");
			Error::as_result()
				.status(401)
				.body(error!(UNAUTHORIZED).to_string())
		}
	} else {
		let result = AccessTokenData::parse(
			authorization.to_string(),
			context.get_state().config.jwt_secret.as_ref(),
		);
		if let Err(err) = result {
			log::warn!("Error occured while parsing JWT: {}", err.to_string());
			return Error::as_result()
				.status(401)
				.body(error!(UNAUTHORIZED).to_string());
		}
		let access_data = result.unwrap();
		Ok(TokenData::AccessTokenData(access_data))
	}
}

async fn validate_access_token(
	redis_conn: &mut RedisConnection,
	access_token: &AccessTokenData,
) -> Result<(), Error> {
	// check whether access token has expired
	if access_token.exp < Utc::now() {
		return Error::as_result()
			.status(401)
			.body(error!(EXPIRED).to_string())?;
	}

	// check whether access token has revoked
	match is_access_token_revoked(redis_conn, access_token).await {
		Ok(false) => (), // access token not revoked hence valid
		_ => {
			// either access token revoked or redis connection error
			return Error::as_result()
				.status(401)
				.body(error!(EXPIRED).to_string());
		}
	}

	Ok(())
}

async fn validate_api_token(api_token: &ApiTokenData) -> Result<(), Error> {
	// check whether access token has expired
	match &api_token.exp {
		Some(exp) => {
			if exp.timestamp_millis() > Utc::now().timestamp_millis() {
				return Error::as_result()
					.status(401)
					.body(error!(EXPIRED).to_string())?;
			}
		}
		None => (),
	}

	Ok(())
}

fn has_permission_to_access_resource(
	resource: &Resource,
	permission_required: &str,
	token_data: TokenData,
) -> bool {
	match token_data {
		TokenData::AccessTokenData(token_data) => {
			let workspace_id = &resource.owner_id;
			let workspace_permission = if let Some(permission) =
				token_data.workspaces.get(workspace_id)
			{
				permission
			} else {
				return false;
			};

			let allowed = {
				// Check if the resource type is allowed
				if let Some(permissions) = workspace_permission
					.resource_types
					.get(&resource.resource_type_id)
				{
					permissions.contains(
						rbac::PERMISSIONS
							.get()
							.unwrap()
							.get(&(*permission_required).to_string())
							.unwrap(),
					)
				} else {
					false
				}
			} || {
				// Check if that specific resource is allowed
				if let Some(permissions) =
					workspace_permission.resources.get(&resource.id)
				{
					permissions.contains(
						rbac::PERMISSIONS
							.get()
							.unwrap()
							.get(&(*permission_required).to_string())
							.unwrap(),
					)
				} else {
					false
				}
			} || {
				// Check if super admin or god is permitted
				workspace_permission.is_super_admin || {
					let god_user_id = GOD_USER_ID.get().unwrap();
					god_user_id == &token_data.user.id
				}
			};
			allowed
		}
		TokenData::ApiTokenData(token_data) => {
			let workspace_id = &resource.owner_id;
			let workspace_permission = if let Some(permission) =
				token_data.workspaces.get(workspace_id)
			{
				permission
			} else {
				log::warn!("Unable to parse workspace permission");
				return false;
			};
			let allowed = {
				// Check if the resource type is allowed
				if let Some(permissions) = workspace_permission
					.resource_types
					.get(&resource.resource_type_id)
				{
					permissions.contains(
						rbac::PERMISSIONS
							.get()
							.unwrap()
							.get(&(*permission_required).to_string())
							.unwrap(),
					)
				} else {
					false
				}
			} || {
				// Check if that specific resource is allowed
				if let Some(permissions) =
					workspace_permission.resources.get(&resource.id)
				{
					permissions.contains(
						rbac::PERMISSIONS
							.get()
							.unwrap()
							.get(&(*permission_required).to_string())
							.unwrap(),
					)
				} else {
					false
				}
			} || {
				// Check if super admin or god is permitted
				workspace_permission.is_super_admin || {
					let god_user_id = GOD_USER_ID.get().unwrap();
					god_user_id == &token_data.user_id
				}
			};
			allowed
		}
	}
}
