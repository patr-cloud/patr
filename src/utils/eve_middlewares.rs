use super::EveContext;
use crate::{
	app::App,
	models::{
		access_token_data::AccessTokenData,
		errors::{error_ids, error_messages},
	},
	utils::constants::request_keys,
};
use express_rs::{
	default_middlewares::{
		compression::CompressionHandler,
		cookie_parser::parser as cookie_parser,
		json::parser as json_parser,
		logger,
		static_file_server::StaticFileServer,
		url_encoded::parser as url_encoded_parser,
	},
	App as EveApp,
	Context,
	Error,
	Middleware,
	NextHandler,
};
use serde_json::json;
use std::{future::Future, pin::Pin};

type MiddlewareHandlerFunction =
	fn(
		EveContext,
		NextHandler<EveContext>,
	) -> Pin<Box<dyn Future<Output = Result<EveContext, Error<EveContext>>> + Send>>;
type ResourcesRequiredFn = fn(EveContext) -> Vec<String>;

#[allow(dead_code)]
#[derive(Clone)]
pub enum EveMiddleware {
	Logger(String),
	Compression(u32),
	JsonParser,
	UrlEncodedParser,
	CookieParser,
	StaticHandler(StaticFileServer),
	TokenAuthenticator(Vec<&'static str>),
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
			EveMiddleware::Logger(format) => {
				let mut logger = logger::with_format(format);
				logger.begin_measuring();
				context = next(context).await?;
				log::info!(
					"{}",
					logger
						.complete_measuring(&context)
						.unwrap_or_else(|| "-".to_string())
				);
				Ok(context)
			}
			EveMiddleware::Compression(compression_level) => {
				let mut compressor = CompressionHandler::create(*compression_level);

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
			EveMiddleware::TokenAuthenticator(_allowed_groups) => {
				let authorization = context.get_request().get_header("Authorization");
				if authorization.is_none() {
					// 401
					context.status(401).json(json!({
						request_keys::SUCCESS: false,
						request_keys::ERROR: error_ids::UNAUTHORIZED,
						request_keys::MESSAGE: error_messages::UNAUTHORIZED
					}));
					return Ok(context);
				}
				let authorization = authorization.unwrap();

				let result = AccessTokenData::parse(
					authorization,
					context.get_state().config.jwt_secret.as_ref(),
				);
				if let Err(err) = result {
					log::warn!("Error occured while parsing JWT: {}", err.to_string());
					context.status(401).json(json!({
						request_keys::SUCCESS: false,
						request_keys::ERROR: error_ids::UNAUTHORIZED,
						request_keys::MESSAGE: error_messages::UNAUTHORIZED
					}));
					return Ok(context);
				}
				let access_data = result.unwrap();

				// TODO check if the user, based on the access token data, is allowed as per allowed_groups

				context.set_token_data(access_data);
				next(context).await
			}
			EveMiddleware::CustomFunction(function) => function(context, next).await,
			EveMiddleware::DomainRouter(domain, app) => {
				if &context.get_host() == domain || &context.get_host() == "localhost" {
					app.resolve(context).await
				} else {
					next(context).await
				}
			}
		}
	}
}
