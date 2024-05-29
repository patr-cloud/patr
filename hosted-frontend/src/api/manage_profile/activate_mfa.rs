use models::api::user::ActivateMfaResponse;

use crate::prelude::*;

/// Sever Function to Activate MFA
#[server(ActivateMfaFn, endpoint = "/user/mfa")]
async fn activate_mfa(
	access_token: Option<String>,
	otp: String,
) -> Result<ActivateMfaResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use models::api::user::{ActivateMfaPath, ActivateMfaRequest, ActivateMfaRequestHeaders};

	let api_response = make_api_call::<ActivateMfaRequest>(
		ApiRequest::builder()
			.path(ActivateMfaPath)
			.query(())
			.headers(ActivateMfaRequestHeaders {
				authorization: BearerToken::from_str(
					format!("{}", access_token.unwrap_or_default()).as_str(),
				)
				.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?,
			})
			.body(ActivateMfaRequest { otp })
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(ServerFnError::WrappedServerError)
}
