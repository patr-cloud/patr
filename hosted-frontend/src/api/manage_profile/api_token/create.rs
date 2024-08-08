use models::api::user::*;

use crate::prelude::*;

#[server(CreateApiTokenFn, endpoint = "/user/api-token/create")]
pub async fn create_api_token(
	access_token: Option<String>,
	token_name: String,
	token_exp: String,
	token_nbf: String,
) -> Result<CreateApiTokenResponse, ServerFnError<ErrorType>> {
	use std::{collections::BTreeMap, str::FromStr};

	use models::{api::user::*, rbac::WorkspacePermission};
	use time::{
		macros::{datetime, format_description},
		Date,
		OffsetDateTime,
	};

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let format = format_description!("[year]-[month]-[day]");

	let token_nbf = token_nbf
		.some_if_not_empty()
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
		.some_if_not_empty()
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

	logging::log!("date tokens: {:?} {:?}", token_exp, token_nbf);

	let token = UserApiToken {
		name: token_name,
		token_exp,
		token_nbf,
		allowed_ips: None,
		created: OffsetDateTime::now_utc(),
		permissions: BTreeMap::<Uuid, WorkspacePermission>::new(),
	};

	let api_response = make_api_call::<CreateApiTokenRequest>(
		ApiRequest::builder()
			.path(CreateApiTokenPath)
			.query(())
			.headers(CreateApiTokenRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(CreateApiTokenRequest { token })
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
