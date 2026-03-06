use models::{ApiSuccessResponseBody, api::user::*};

use crate::prelude::*;

#[tokio::test]
async fn get_mfa_secret_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_api_call(
			ApiRequest::<GetMfaSecretRequest>::builder()
				.headers(GetMfaSecretRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetMfaSecretResponse>>();

	assert!(
		!response.response.qr.is_empty(),
		"QR URL should not be empty"
	);
}

#[tokio::test]
async fn activate_mfa_wrong_otp() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	// Get the secret first
	let _secret = setup
		.make_api_call(
			ApiRequest::<GetMfaSecretRequest>::builder()
				.headers(GetMfaSecretRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetMfaSecretResponse>>();

	// Try activating with wrong OTP
	let response = setup
		.make_api_call(
			ApiRequest::<ActivateMfaRequest>::builder()
				.headers(ActivateMfaRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ActivateMfaRequest {
					otp: "000000".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for wrong MFA OTP"
	);
}

#[tokio::test]
async fn mfa_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_api_call(
			ApiRequest::<GetMfaSecretRequest>::builder()
				.headers(GetMfaSecretRequestHeaders {
					authorization: BearerToken::from_str("invalid-token").unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error without auth token"
	);
}
