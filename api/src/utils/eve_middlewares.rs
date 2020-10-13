use crate::{
	app::App,
	models::{db_mapping::Resource, error, rbac, AccessTokenData},
	utils::{constants::request_keys, get_current_time, EveContext},
};
use eve_rs::{
	default_middlewares::{
		compression::CompressionHandler,
		cookie_parser::parser as cookie_parser, json::parser as json_parser,
		static_file_server::StaticFileServer,
		url_encoded::parser as url_encoded_parser,
	},
	App as EveApp, Context, Error, Middleware, NextHandler,
};
use rbac::GOD_USER_ID;
use serde_json::json;
use std::{future::Future, pin::Pin};

pub type MiddlewareHandlerFunction = fn(
	EveContext,
	NextHandler<EveContext>,
) -> Pin<
	Box<dyn Future<Output = Result<EveContext, Error<EveContext>>> + Send>,
>;

pub type ResourceRequiredFunction = fn(
	EveContext,
) -> Pin<
	Box<
		dyn Future<
				Output = Result<
					(EveContext, Option<Resource>),
					Error<EveContext>,
				>,
			> + Send,
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
	DomainRouter(String, Box<EveApp<EveContext, EveMiddleware, App>>),
}

#[async_trait]
impl Middleware<EveContext> for EveMiddleware {
	async fn run_middleware(
		&self,
		mut context: EveContext,
		next: NextHandler<EveContext>,
	) -> Result<EveContext, Error<EveContext>> {
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
					if let Some(token) = decode_access_token(&mut context) {
						token
					} else {
						context.status(401).json(json!({
							request_keys::SUCCESS: false,
							request_keys::ERROR: error::id::UNAUTHORIZED,
							request_keys::MESSAGE: error::message::UNAUTHORIZED
						}));
						return Ok(context);
					};

				if !is_access_token_valid(&access_data) {
					context.status(401).json(json!({
						request_keys::SUCCESS: false,
						request_keys::ERROR: error::id::UNAUTHORIZED,
						request_keys::MESSAGE: error::message::UNAUTHORIZED
					}));
					return Ok(context);
				}

				context.set_token_data(access_data);
				next(context).await
			}
			EveMiddleware::ResourceTokenAuthenticator(
				permission_required,
				resource_in_question,
			) => {
				let access_data =
					if let Some(token) = decode_access_token(&mut context) {
						token
					} else {
						context.status(401).json(json!({
							request_keys::SUCCESS: false,
							request_keys::ERROR: error::id::UNAUTHORIZED,
							request_keys::MESSAGE: error::message::UNAUTHORIZED
						}));
						return Ok(context);
					};

				if !is_access_token_valid(&access_data) {
					context.status(401).json(json!({
						request_keys::SUCCESS: false,
						request_keys::ERROR: error::id::UNAUTHORIZED,
						request_keys::MESSAGE: error::message::UNAUTHORIZED
					}));
					return Ok(context);
				}

				// check if the access token has access to the resource
				let (mut context, resource) =
					resource_in_question(context).await?;
				if resource.is_none() {
					context.status(404).json(json!({
						request_keys::SUCCESS: false,
						request_keys::ERROR: error::id::RESOURCE_DOES_NOT_EXIST,
						request_keys::MESSAGE: error::message::RESOURCE_DOES_NOT_EXIST,
					}));
					return Ok(context);
				}
				let resource = resource.unwrap();

				let org_id = hex::encode(resource.owner_id);
				let org_permission = access_data.orgs.get(&org_id);

				if org_permission.is_none() {
					context.status(404).json(json!({
						request_keys::SUCCESS: false,
						request_keys::ERROR: error::id::RESOURCE_DOES_NOT_EXIST,
						request_keys::MESSAGE: error::message::RESOURCE_DOES_NOT_EXIST,
					}));
					return Ok(context);
				}
				let org_permission = org_permission.unwrap();

				let allowed = if let Some(permissions) = org_permission
					.resource_types
					.get(&resource.resource_type_id)
				{
					permissions.contains(&permission_required.to_string())
				} else if let Some(permissions) =
					org_permission.resources.get(&resource.id)
				{
					permissions.contains(&permission_required.to_string())
				} else {
					org_permission.is_super_admin || {
						let god_user_id = GOD_USER_ID.get().unwrap().as_bytes();
						access_data.user.id == god_user_id
					}
				};

				if allowed {
					context.set_token_data(access_data);
					next(context).await
				} else {
					context.status(401).json(json!({
						request_keys::SUCCESS: false,
						request_keys::ERROR: error::id::UNPRIVILEGED,
						request_keys::MESSAGE: error::message::UNPRIVILEGED,
					}));
					Ok(context)
				}
			}
			EveMiddleware::CustomFunction(function) => {
				function(context, next).await
			}
			EveMiddleware::DomainRouter(domain, app) => {
				if context.get_host()
					== format!("localhost:{}", app.get_state().config.port)
					|| &context.get_host() == domain
				{
					app.resolve(context).await
				} else {
					next(context).await
				}
			}
		}
	}
}

fn decode_access_token(context: &mut EveContext) -> Option<AccessTokenData> {
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

fn is_access_token_valid(token: &AccessTokenData) -> bool {
	// TODO token banning goes here
	token.exp > get_current_time()
}
