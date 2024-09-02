use std::collections::BTreeMap;

use leptos::server_fn::codec::Json;
use models::{api::user::*, rbac::WorkspacePermission};

use crate::prelude::*;

#[server(UpdateApiTokenFn, endpoint = "/user/api-token/update", input = Json)]
pub async fn update_api_token(
	access_token: Option<String>,
	token_id: String,
	token_name: Option<String>,
	token_exp: Option<String>,
	token_nbf: Option<String>,
	permissions: Option<BTreeMap<Uuid, WorkspacePermission>>,
) -> Result<UpdateApiTokenResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use time::{
		macros::{datetime, format_description},
		Date,
		OffsetDateTime,
	};

	logging::log!(
		"{:#?} {:?} {:?} {:?} {:?}",
		permissions,
		token_id,
		token_name,
		token_exp,
		token_nbf
	);

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let token_id = Uuid::parse_str(token_id.clone().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let format = format_description!("[year]-[month]-[day]");

	let token_nbf = token_nbf
		.map(|nbf| {
			let date = Date::parse(nbf.as_str(), &format).map_err(|er| {
				logging::log!("{:#?}", er);
				ServerFnError::WrappedServerError(ErrorType::WrongParameters)
			})?;

			Ok::<OffsetDateTime, ServerFnError<ErrorType>>(
				datetime!(2020-01-01 0:00 UTC).replace_date(date),
			)
		})
		.transpose()?;

	let token_exp = token_exp
		.map(|exp| {
			let date = Date::parse(exp.as_str(), &format).map_err(|er| {
				logging::log!("{:#?}", er);
				ServerFnError::WrappedServerError(ErrorType::WrongParameters)
			})?;

			Ok::<OffsetDateTime, ServerFnError<ErrorType>>(
				datetime!(2020-01-01 0:00 UTC).replace_date(date),
			)
		})
		.transpose()?;

	let update_request_body = UpdateApiTokenRequest {
		name: token_name,
		token_exp,
		token_nbf,
		permissions,
		allowed_ips: None,
	};

	let api_response = make_api_call::<UpdateApiTokenRequest>(
		ApiRequest::builder()
			.path(UpdateApiTokenPath { token_id })
			.query(())
			.headers(UpdateApiTokenRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_str("hyper/0.12.2").unwrap(),
			})
			.body(update_request_body)
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|err| ServerFnError::WrappedServerError(err))
}
