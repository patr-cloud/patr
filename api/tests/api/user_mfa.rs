use http::header;
use models::{
	ApiSuccessResponseBody,
	api::{ApiEndpoint, user::*},
};

use crate::prelude::*;

#[tokio::test]
async fn get_mfa_secret_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;

	let response = setup
		.server
		.method(
			GetMfaSecretRequest::METHOD,
			&GetMfaSecretPath.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<GetMfaSecretResponse>>();

	assert!(!response.response.qr.is_empty(), "QR URL should not be empty");
}

#[tokio::test]
async fn activate_mfa_wrong_otp() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;

	// Get the secret first
	let _secret = setup
		.server
		.method(
			GetMfaSecretRequest::METHOD,
			&GetMfaSecretPath.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<GetMfaSecretResponse>>();

	// Try activating with wrong OTP
	let response = setup
		.server
		.method(
			ActivateMfaRequest::METHOD,
			&ActivateMfaPath.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.json(&ActivateMfaRequest {
			otp: "000000".to_string(),
		})
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
		.server
		.method(
			GetMfaSecretRequest::METHOD,
			&GetMfaSecretPath.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error without auth token"
	);
}
