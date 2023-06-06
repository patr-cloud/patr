use std::{
	future::Future,
	net::{IpAddr, Ipv4Addr},
	pin::Pin,
};

use api_models::utils::Uuid;
use async_trait::async_trait;
use eve_rs::{
	App as EveApp,
	AsError,
	Context,
	Error as _,
	Middleware,
	NextHandler,
};
use reqwest::header::HeaderName;

use crate::{
	app::App,
	db::Resource,
	error,
	models::UserAuthenticationData,
	routes::get_request_ip_address,
	utils::{Error, EveContext},
};

pub type MiddlewareHandlerFunction =
	fn(
		EveContext,
		NextHandler<EveContext, Error>,
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
	JsonParser,
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
	DomainRouter(String, Box<EveApp<EveContext, EveMiddleware, App, Error>>),
}

#[async_trait]
impl Middleware<EveContext, Error> for EveMiddleware {
	async fn run_middleware(
		&self,
		mut context: EveContext,
		next: NextHandler<EveContext, Error>,
	) -> Result<EveContext, Error> {
		match self {
			EveMiddleware::JsonParser => {
				if context.get_content_type() == "application/json" {
					let body = context.get_body().await?;
					context.set_body_object(
						serde_json::from_str(&body).status(400)?,
					);
				}

				next(context).await
			}
			EveMiddleware::PlainTokenAuthenticator {
				is_api_token_allowed,
			} => {
				let token = context
					.get_header(HeaderName::from_static("authorization"))
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
					return Err(Error::from_msg("Unauthorized")
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
					.get_header(HeaderName::from_static("Authorization"))
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
					.get_header(HeaderName::from_static("authorization"))
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
					return Err(Error::from_msg("Unauthorized")
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
					return Err(Error::from_msg("Unauthorized")
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
