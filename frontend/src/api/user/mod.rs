use models::api::user::*;

use crate::prelude::*;

mod activate_mfa;
mod api_token;
mod change_passsword;

pub use self::{activate_mfa::*, api_token::*, change_passsword::*};

/// Load user data from the server
#[server]
pub async fn load_user_data(
	access_token: Option<String>,
) -> Result<GetUserInfoResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	make_api_call::<GetUserInfoRequest>(
		ApiRequest::builder()
			.path(GetUserInfoPath)
			.query(())
			.headers(GetUserInfoRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(GetUserInfoRequest)
			.build(),
	)
	.await
	.map(|res| res.body)
	.map_err(ServerFnError::WrappedServerError)
}
