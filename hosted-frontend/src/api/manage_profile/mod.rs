use models::api::user::GetUserInfoResponse;

use crate::prelude::*;

mod activate_mfa;
mod change_passsword;

pub use self::{activate_mfa::*, change_passsword::*};

/// Load user data from the server
#[server]
pub async fn load_user_data(
	access_token: Option<String>,
) -> Result<GetUserInfoResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use models::api::user::{GetUserInfoPath, GetUserInfoRequest, GetUserInfoRequestHeaders};

	let api_response = make_api_call::<GetUserInfoRequest>(
		ApiRequest::builder()
			.path(GetUserInfoPath)
			.query(())
			.headers(GetUserInfoRequestHeaders {
				authorization: BearerToken::from_str(
					format!("Bearer {}", access_token.unwrap_or_default()).as_str(),
				)
				.map_err(|e| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?,
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(GetUserInfoRequest)
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(ServerFnError::WrappedServerError)
}
