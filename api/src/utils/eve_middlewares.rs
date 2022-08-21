use std::{collections::HashMap, future::Future, pin::Pin};

use api_models::utils::{DateTime, Uuid};
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
				let (access_token_data, api_token_data) =
					decode_access_token(&context)
						.await?
						.status(401)
						.body(error!(UNAUTHORIZED).to_string())?;

				if access_token_data {
					validate_access_token(
						context.get_redis_connection(),
						&access_token_data,
					)
					.await?;
					context.set_token_data(access_token_data);
				} else if api_token_data {
					validate_api_token(
						context.get_redis_connection(),
						&api_token_data,
					)
					.await?;
					context.set_token_data(api_token_data);
				} else {
					return Err(error!(UNAUTHORIZED));
				}

				next(context).await
			}
			EveMiddleware::ResourceTokenAuthenticator(
				permission_required,
				resource_in_question,
			) => {
				let (access_token_data, api_token_data) =
					decode_access_token(&context)
						.status(401)
						.body(error!(UNAUTHORIZED).to_string())?;

				let (mut context, resource) =
					resource_in_question(context).await?;
				let resource = if let Some(resource) = resource {
					resource
				} else {
					return Ok(context);
				};

				if access_token_data {
					validate_access_token(
						context.get_redis_connection(),
						&access_token_data,
					)
					.await?;
					context.set_token_data(access_token_data);

					let allowed = has_permission_to_access_resource(
						&context,
						&resource,
						permission_required,
						Some(access_token_data),
						Some(api_token_data),
					);

					if allowed {
						context.set_token_data(access_token_data);
						next(context).await
					} else {
						context.status(401).json(error!(UNAUTHORIZED));
						Ok(context)
					}
				} else if api_token_data {
					validate_api_token(
						context.get_redis_connection(),
						&api_token_data,
					)
					.await?;
					context.set_token_data(api_token_data);
					let allowed = has_permission_to_access_resource(
						&context,
						&resource,
						permission_required,
						Some(access_token_data),
						Some(api_token_data),
					);

					if allowed {
						context.set_token_data(api_token_data);
						next(context).await
					} else {
						context.status(401).json(error!(UNAUTHORIZED));
						Ok(context)
					}
				} else {
					return Err(error!(UNAUTHORIZED));
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
		}
	}
}

async fn decode_access_token(
	context: &EveContext,
) -> (Option<AccessTokenData>, Option<ApiTokenData>) {
	let authorization = context
		.get_header("Authorization")
		.unwrap_or("".to_string());
	let is_api_token = is_uuid(&authorization);

	if is_api_token {
		let token = Uuid::parse_str(&authorization).unwrap();
		let api_token = db::get_api_token_by_id(
			context.get_database_connection(),
			&token,
		)
		.await?;
		// .ok();

		if let Some(api_token) = api_token {
			let is_token_valid = if let Some(expiry) = api_token.token_expiry {
				expiry > DateTime::<Utc>::now()
			} else {
				true
			};
			if !is_token_valid {
				return Error::as_result().status(401).body(error!(UNAUTHORIZED).to_string())?;
			}
			let api_token_permissions = db::list_permissions_for_api_token(
				context.get_database_connection(),
				&token,
			)
			.await
			.ok()?;

			let workspace_id = db::get_workspace_id_for_api_token(
				context.get_database_connection(),
				&token,
			)
			.await
			.ok()?;

			if workspace_id.is_none() {
				log::warn!("Cannot get workspace_id for api_token:{}", token);
				return Error::as_result()
					.status(401)
					.body(error!(UNAUTHORIZED).to_string())
					.ok()?;
			}

			let api_token_data = ApiTokenData {
				is_super_admin: api_token.is_super_admin,
				exp: api_token.token_expiry,
				workspaces: HashMap::from([(
					workspace_id.unwrap(),
					WorkspacePermissions {
						is_super_admin: api_token.is_super_admin,
						resources: api_token_permissions.resource_permissions,
						resource_types: api_token_permissions
							.resource_type_permissions,
					},
				)]),
				user_id: api_token.user_id,
			};
			(None, Some(api_token_data))
		} else {
			log::warn!("Invalid api token");
			return None;
		}
	} else {
		let result = AccessTokenData::parse(
			authorization,
			context.get_state().config.jwt_secret.as_ref(),
		);
		if let Err(err) = result {
			log::warn!("Error occured while parsing JWT: {}", err.to_string());
			return None;
		}
		let access_data = result.unwrap();
		(Some(access_data), None)
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

async fn validate_api_token(
	redis_conn: &mut RedisConnection,
	api_token: &ApiTokenData,
) -> Result<(), Error> {
	// check whether access token has expired
	match api_token.exp {
		Some(exp) => {
			if exp < DateTime::<Utc>::now() {
				return Error::as_result()
					.status(401)
					.body(error!(EXPIRED).to_string())?;
			}
		}
		None => Ok(()),
	}

	Ok(())
}

fn is_uuid(id: &str) -> bool {
	let result = match Uuid::parse_str(id) {
		Ok(id) => true,
		Err(_) => false,
	};
	result
}

fn has_permission_to_access_resource(
	context: &EveContext,
	resource: &Resource,
	permission_required: &str,
	access_token_data: Option<AccessTokenData>,
	api_token_data: Option<ApiTokenData>,
) -> bool {
	if let Some(access_token_data) = access_token_data {
		let workspace_id = resource.owner_id;
		let workspace_permission = if let Some(permission) =
			access_token_data.workspaces.get(&workspace_id)
		{
			permission
		} else {
			return false				
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
				god_user_id == &access_token_data.user.id
			}
		};
		return allowed;
	} else if let Some(api_token_data) = api_token_data {
		let workspace_id = resource.owner_id;
		let workspace_permission = if let Some(permission) =
			api_token_data.workspaces.get(&workspace_id)
		{
			permission
		} else {
			return false
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
				god_user_id == &api_token_data.user_id
			}
		};
		return allowed;
	} else {
		return false;
	};
}
