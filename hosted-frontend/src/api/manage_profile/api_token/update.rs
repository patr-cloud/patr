use std::collections::BTreeMap;

use models::api::user::*;

use crate::prelude::*;

#[server(UpdateApiTokenFn, endpoint = "/user/api-token/update")]
pub async fn update_api_token(
	access_token: Option<String>,
	token_id: String,
	token_name: Option<String>,
	token_exp: Option<String>,
	token_nbf: Option<String>,
	super_admin: Option<Vec<String>>,
) -> Result<UpdateApiTokenResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use models::rbac::WorkspacePermission;
	use time::{
		macros::{datetime, format_description},
		Date,
		OffsetDateTime,
	};

	logging::log!(
		"{:#?} {:?} {:?} {:?} {:?}",
		super_admin,
		token_id,
		token_name,
		token_exp,
		token_nbf
	);

	let super_admin = super_admin.map(|admins| {
		let mut permissions = BTreeMap::<Uuid, WorkspacePermission>::new();

		admins.iter().for_each(|perm| {
			let workspace_id = Uuid::parse_str(perm).unwrap();
			permissions.insert(workspace_id, WorkspacePermission::SuperAdmin);
		});

		permissions
	});

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
		allowed_ips: None,
		permissions: super_admin,
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
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
