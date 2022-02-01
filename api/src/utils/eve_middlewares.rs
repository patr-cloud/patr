use std::{future::Future, pin::Pin};

use async_trait::async_trait;
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
use redis::AsyncCommands;

use crate::{
	app::App,
	error,
	models::{
		db_mapping::Resource,
		rbac::{self, GOD_USER_ID},
		AccessTokenData,
	},
	utils::{get_current_time_millis, Error, ErrorData, EveContext},
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
				let access_data =
					if let Some(token) = decode_access_token(&context) {
						token
					} else {
						context.status(401).json(error!(UNAUTHORIZED));
						return Ok(context);
					};

				is_access_token_valid(
					&access_data,
					&context.get_header("Authorization").unwrap(),
					context.get_state_mut(),
				)
				.await?;

				context.set_token_data(access_data);
				next(context).await
			}
			EveMiddleware::ResourceTokenAuthenticator(
				permission_required,
				resource_in_question,
			) => {
				let access_data = decode_access_token(&context)
					.status(401)
					.body(error!(UNAUTHORIZED).to_string())?;

				is_access_token_valid(
					&access_data,
					&context.get_header("Authorization").unwrap(),
					context.get_state_mut(),
				)
				.await?;

				// check if the access token has access to the resource
				let (mut context, resource) =
					resource_in_question(context).await?;
				let resource = if let Some(resource) = resource {
					resource
				} else {
					return Ok(context);
				};

				let workspace_id = resource.owner_id;
				let workspace_permission = if let Some(permission) =
					access_data.workspaces.get(&workspace_id)
				{
					permission
				} else {
					context.status(404).json(error!(RESOURCE_DOES_NOT_EXIST));
					return Ok(context);
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
						god_user_id == &access_data.user.id
					}
				};

				if allowed {
					context.set_token_data(access_data);
					next(context).await
				} else {
					context.status(401).json(error!(UNPRIVILEGED));
					Ok(context)
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

fn decode_access_token(context: &EveContext) -> Option<AccessTokenData> {
	let authorization = context.get_header("Authorization")?;

	let result = AccessTokenData::parse(
		authorization,
		context.get_state().config.jwt_secret.as_ref(),
	);
	if let Err(err) = result {
		log::warn!("Error occured while parsing JWT: {}", err.to_string());
		return None;
	}
	let access_data = result.unwrap();
	Some(access_data)
}

async fn is_access_token_valid(
	token: &AccessTokenData,
	token_string: &str,
	app: &mut App,
) -> Result<(), Error> {
	// token banning goes here
	// Different types of banned tokens:
	// - Specific tokens
	// - User IDs whose tokens after a given timestamp is invalid
	// - Global timestamp after which all tokens are invalid
	if token.exp < get_current_time_millis() {
		// If current time is more than expiry, return false
		return Error::as_result()
			.status(401)
			.body(error!(EXPIRED).to_string())?;
	}

	let token_banned: Option<String> = app.redis.get(token_string).await?;
	if token_banned.is_some() {
		// This token is banned. Invalidate it
		return Error::as_result()
			.status(401)
			.body(error!(UNAUTHORIZED).to_string())?;
	}

	let user_exp: Option<u64> = app
		.redis
		.get(format!("user-{}-exp", token.user.id.as_str()))
		.await?;
	if let Some(exp) = user_exp {
		if exp < get_current_time_millis() {
			// This user needs an exp greater than user-userid-exp
			return Error::as_result()
				.status(401)
				.body(error!(EXPIRED).to_string())?;
		}
	}

	let global_exp: Option<u64> = app.redis.get("global-user-exp").await?;
	if let Some(exp) = global_exp {
		if exp < get_current_time_millis() {
			// This user needs an exp greater than global-user-exp
			return Error::as_result()
				.status(401)
				.body(error!(EXPIRED).to_string())?;
		}
	}

	Ok(())
}
