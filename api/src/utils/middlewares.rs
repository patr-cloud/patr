use std::{
	future::Future,
	net::{IpAddr, Ipv4Addr},
	pin::Pin,
};

use api_models::utils::DtoMiddleware;
use axum::{
	headers::{authorization::Bearer, Authorization},
	RequestExt,
	TypedHeader,
};
use http::Request as HttpRequest;

use crate::{db::Resource, models::UserAuthenticationData, prelude::*};

// #[derive(Clone)]
// pub enum EveMiddleware {
// 	PlainTokenAuthenticator {
// 		is_api_token_allowed: bool,
// 	},
// 	ResourceTokenAuthenticator {
// 		is_api_token_allowed: bool,
// 		permission: &'static str,
// 		resource: ResourceRequiredFunction,
// 	},
// 	CustomFunction(MiddlewareHandlerFunction),
// }

// #[async_trait]
// impl Middleware<EveContext, ErrorData> for EveMiddleware {
// 	async fn run_middleware(
// 		&self,
// 		mut context: EveContext,
// 		next: NextHandler<EveContext, ErrorData>,
// 	) -> Result<EveContext, Error> {
// 		match self {
// 			EveMiddleware::PlainTokenAuthenticator {
// 				is_api_token_allowed,
// 			} => {
// 				let token = context
// 					.get_header("Authorization")
// 					.status(401)
// 					.body(error!(UNAUTHORIZED).to_string())?;

// 				let ip_addr = get_request_ip_address(&context)
// 					.parse()
// 					.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

// 				let jwt_secret = context.get_state().config.jwt_secret.clone();
// 				let mut redis_conn = context.get_redis_connection().clone();
// 				let token_data = UserAuthenticationData::parse(
// 					context.get_database_connection(),
// 					&mut redis_conn,
// 					&jwt_secret,
// 					&token,
// 					&ip_addr,
// 				)
// 				.await?;

// 				if token_data.is_api_token() && !is_api_token_allowed {
// 					return Err(Error::empty()
// 						.status(401)
// 						.body(error!(UNAUTHORIZED).to_string()));
// 				}

// 				context.set_token_data(token_data);
// 				next(context).await
// 			}
// 			EveMiddleware::ResourceTokenAuthenticator {
// 				permission,
// 				resource: resource_in_question,
// 				is_api_token_allowed,
// 			} => {
// 				let token = context
// 					.get_header("Authorization")
// 					.status(401)
// 					.body(error!(UNAUTHORIZED).to_string())?;

// 				let ip_addr = get_request_ip_address(&context)
// 					.parse()
// 					.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

// 				let jwt_secret = context.get_state().config.jwt_secret.clone();
// 				let mut redis_conn = context.get_redis_connection().clone();
// 				let token_data = UserAuthenticationData::parse(
// 					context.get_database_connection(),
// 					&mut redis_conn,
// 					&jwt_secret,
// 					&token,
// 					&ip_addr,
// 				)
// 				.await?;

// 				if token_data.is_api_token() && !is_api_token_allowed {
// 					return Err(Error::empty()
// 						.status(401)
// 						.body(error!(UNAUTHORIZED).to_string()));
// 				}

// 				let (mut context, resource) =
// 					resource_in_question(context).await?;
// 				let resource = if let Some(resource) = resource {
// 					resource
// 				} else {
// 					return Ok(context);
// 				};

// 				if !token_data.has_access_for_requested_action(
// 					&resource.owner_id,
// 					&resource.id,
// 					&resource.resource_type_id,
// 					permission,
// 				) {
// 					return Err(Error::empty()
// 						.status(401)
// 						.body(error!(UNAUTHORIZED).to_string()));
// 				}

// 				context.set_token_data(token_data);
// 				next(context).await
// 			}
// 			EveMiddleware::CustomFunction(function) => {
// 				function(context, next).await
// 			}
// 		}
// 	}
// }

#[derive(Debug, Clone)]
pub struct PlainTokenAuthenticator {
	pub is_api_token_allowed: bool,
}

impl PlainTokenAuthenticator {
	pub fn new() -> Self {
		Self {
			is_api_token_allowed: true,
		}
	}

	pub fn disallow_api_token(mut self) -> Self {
		self.is_api_token_allowed = false;
		self
	}
}

impl<Req, B> DtoMiddleware<Req, App, B> for PlainTokenAuthenticator
where
	Req: ApiRequest,
	B: Send,
{
	type Future =
		Pin<Box<dyn Future<Output = Result<HttpRequest<B>, Error>> + Send>>;

	fn run(
		self,
		path: <Req as ApiRequest>::Path,
		query: <Req as ApiRequest>::Query,
		state: App,
		req: HttpRequest<B>,
	) -> Self::Future {
		Box::pin(async move {
			let TypedHeader(token) = req
				.extract_parts::<TypedHeader<Authorization<Bearer>>>()
				.await
				.map_err(|err| {
					todo!("Decide on what error is thrown when tokens are invalid");
					ErrorType::Unauthorized
				})
				.status(StatusCode::UNAUTHORIZED)?;
			let ip_addr = get_request_ip_address(&req)
				.parse()
				.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

			let token_data = UserAuthenticationData::parse(
				&mut state.database,
				&mut state.redis,
				&state.config.jwt_secret,
				&token,
				&ip_addr,
			)
			.await?;

			if token_data.is_api_token() && !self.is_api_token_allowed {
				return Err(Error::new(ErrorType::Unauthorized));
			}

			let req = req.set_token_data(token_data);
			Ok(req)
		})
	}
}

#[derive(Debug, Clone)]
pub struct ResourceTokenAuthenticator<F> {
	is_api_token_allowed: bool,
	permission: &'static str,
	resource_in_question: F,
}

impl<TFunc, TRes, Req, B> ResourceTokenAuthenticator<TFunc>
where
	TFunc: Fn(
		<Req as ApiRequest>::Path,
		<Req as ApiRequest>::Query,
		App,
		&HttpRequest<B>,
	) -> TRes,
	TRes: Future<Output = Result<Option<Resource>, Error>>,
	Req: ApiRequest,
	B: Send,
{
	pub fn new(permission: &'static str, resource_in_question: TFunc) -> Self {
		Self {
			is_api_token_allowed: true,
			permission,
			resource_in_question,
		}
	}

	pub fn disallow_api_token(mut self) -> Self {
		self.is_api_token_allowed = false;
		self
	}
}

impl<TFunc, TRes, Req, B> DtoMiddleware<Req, App, B>
	for ResourceTokenAuthenticator<TFunc>
where
	TFunc: Fn(
		<Req as ApiRequest>::Path,
		<Req as ApiRequest>::Query,
		App,
		&HttpRequest<B>,
	) -> TRes,
	TRes: Future<Output = Result<Option<Resource>, Error>>,
	Req: ApiRequest,
	B: Send,
{
	type Future =
		Pin<Box<dyn Future<Output = Result<HttpRequest<B>, Error>> + Send>>;

	fn run(
		self,
		path: <Req as ApiRequest>::Path,
		query: <Req as ApiRequest>::Query,
		state: App,
		req: HttpRequest<B>,
	) -> Self::Future {
		Box::pin(async move {
			let TypedHeader(token) = req
				.extract_parts::<TypedHeader<Authorization<Bearer>>>()
				.await
				.map_err(|err| {
					todo!("Decide on what error is thrown when tokens are invalid");
					ErrorType::Unauthorized
				})
				.status(StatusCode::UNAUTHORIZED)?;
			let ip_addr = get_request_ip_address(&req)
				.parse()
				.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

			let token_data = UserAuthenticationData::parse(
				&mut state.database,
				&mut state.redis,
				&state.config.jwt_secret,
				&token,
				&ip_addr,
			)
			.await?;

			if token_data.is_api_token() && !self.is_api_token_allowed {
				return Err(Error::new(ErrorType::Unauthorized));
			}

			let resource =
				self.resource_in_question(path, query, state, &req).await?;

			let req = req.set_token_data(token_data);
			Ok(req)
		})
	}
}
