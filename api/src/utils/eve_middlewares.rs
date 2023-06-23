use std::{
	future::Future,
	net::{IpAddr, Ipv4Addr},
	pin::Pin,
};

use api_models::utils::Uuid;
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

use crate::{
	app::App,
	db::Resource,
	error,
	models::UserAuthenticationData,
	routes::get_request_ip_address,
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

pub type RequestedWorkspaceFunction = fn(
	EveContext,
) -> Pin<
	Box<dyn Future<Output = Result<(EveContext, Uuid), Error>> + Send>,
>;

#[allow(dead_code)]
#[derive(Clone)]
pub enum EveMiddleware {
	Compression(u32),
	JsonParser,
	UrlEncodedParser,
	CookieParser,
	StaticHandler(StaticFileServer),
	PlainTokenAuthenticator {
		is_api_token_allowed: bool,
	},
	WorkspaceMemberAuthenticator {
		is_api_token_allowed: bool,
		requested_workspace: RequestedWorkspaceFunction,
	},
	ResourceTokenAuthenticator {
		is_api_token_allowed: bool,
		permission: &'static str,
		resource: ResourceRequiredFunction,
	},
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
			EveMiddleware::PlainTokenAuthenticator {
				is_api_token_allowed,
			} => {
				let token = context
					.get_header("Authorization")
					.status(401)
					.body(error!(UNAUTHORIZED).to_string())?;

				let ip_addr = get_request_ip_address(&context)
					.parse()
					.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

				let jwt_secret = context.get_state().config.jwt_secret.clone();
				let mut redis_conn = context.get_redis_connection().clone();
				let token_data = UserAuthenticationData::parse(
					context.get_database_connection(),
					&mut redis_conn,
					&jwt_secret,
					&token,
					&ip_addr,
				)
				.await?;

				if token_data.is_api_token() && !is_api_token_allowed {
					return Err(Error::empty()
						.status(401)
						.body(error!(UNAUTHORIZED).to_string()));
				}

				context.set_token_data(token_data);
				next(context).await
			}
			EveMiddleware::WorkspaceMemberAuthenticator {
				is_api_token_allowed,
				requested_workspace,
			} => {
				let token = context
					.get_header("Authorization")
					.status(401)
					.body(error!(UNAUTHORIZED).to_string())?;

				let ip_addr = get_request_ip_address(&context)
					.parse()
					.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

				let jwt_secret = context.get_state().config.jwt_secret.clone();
				let mut redis_conn = context.get_redis_connection().clone();
				let token_data = UserAuthenticationData::parse(
					context.get_database_connection(),
					&mut redis_conn,
					&jwt_secret,
					&token,
					&ip_addr,
				)
				.await?;

				if token_data.is_api_token() && !is_api_token_allowed {
					return Err(Error::empty()
						.status(401)
						.body(error!(UNAUTHORIZED).to_string()));
				}

				let (mut context, requested_workspace) =
					requested_workspace(context).await?;

				if !token_data
					.has_access_for_requested_workspace(&requested_workspace)
				{
					return Err(Error::empty()
						.status(401)
						.body(error!(UNAUTHORIZED).to_string()));
				}

				context.set_token_data(token_data);
				next(context).await
			}
			EveMiddleware::ResourceTokenAuthenticator {
				permission,
				resource: resource_in_question,
				is_api_token_allowed,
			} => {
				let token = context
					.get_header("Authorization")
					.status(401)
					.body(error!(UNAUTHORIZED).to_string())?;

				let ip_addr = get_request_ip_address(&context)
					.parse()
					.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

				let jwt_secret = context.get_state().config.jwt_secret.clone();
				let mut redis_conn = context.get_redis_connection().clone();
				let token_data = UserAuthenticationData::parse(
					context.get_database_connection(),
					&mut redis_conn,
					&jwt_secret,
					&token,
					&ip_addr,
				)
				.await?;

				if token_data.is_api_token() && !is_api_token_allowed {
					return Err(Error::empty()
						.status(401)
						.body(error!(UNAUTHORIZED).to_string()));
				}

				let (mut context, resource) =
					resource_in_question(context).await?;
				let resource = if let Some(resource) = resource {
					resource
				} else {
					return Ok(context);
				};

				if !token_data.has_access_for_requested_action(
					&resource.owner_id,
					&resource.id,
					permission,
				) {
					return Err(Error::empty()
						.status(401)
						.body(error!(UNAUTHORIZED).to_string()));
				}

				context.set_token_data(token_data);
				next(context).await
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
