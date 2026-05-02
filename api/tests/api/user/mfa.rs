use models::{ApiSuccessResponseBody, api::user::*};

use crate::prelude::*;

#[tokio::test]
async fn get_mfa_secret_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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

/// Helper: activate MFA for `user` using the secret currently stored in
/// Redis. Returns the base32 secret so callers can compute later codes.
async fn activate_mfa_for_user(setup: &TestSetup, user: &TestUser) -> String {
	_ = setup
		.make_web_dashboard_call(
			ApiRequest::<GetMfaSecretRequest>::builder()
				.headers(GetMfaSecretRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetMfaSecretResponse>>();

	let secret = setup
		.get_redis_value(&format!("mfa:{}", user.user_id))
		.await
		.expect("MFA secret should be in redis after get_mfa_secret");

	let otp = setup.compute_totp(&secret);

	setup
		.make_web_dashboard_call(
			ApiRequest::<ActivateMfaRequest>::builder()
				.headers(ActivateMfaRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ActivateMfaRequest { otp })
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(ActivateMfaResponse));

	secret
}

#[tokio::test]
async fn activate_mfa_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let _secret = activate_mfa_for_user(&setup, &user).await;

	// Once activated, get_mfa_secret should reject with MfaAlreadyActive.
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetMfaSecretRequest>::builder()
				.headers(GetMfaSecretRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"get_mfa_secret should fail after activation"
	);
}

#[tokio::test]
async fn deactivate_mfa_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let secret = activate_mfa_for_user(&setup, &user).await;
	let otp = setup.compute_totp(&secret);

	setup
		.make_web_dashboard_call(
			ApiRequest::<DeactivateMfaRequest>::builder()
				.headers(DeactivateMfaRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(DeactivateMfaRequest { otp })
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeactivateMfaResponse));

	// After deactivation, get_mfa_secret should succeed again.
	_ = setup
		.make_web_dashboard_call(
			ApiRequest::<GetMfaSecretRequest>::builder()
				.headers(GetMfaSecretRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetMfaSecretResponse>>();
}

#[tokio::test]
async fn activate_mfa_when_already_active() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let secret = activate_mfa_for_user(&setup, &user).await;

	// Second activate attempt with a fresh valid OTP should still fail —
	// the handler's MfaAlreadyActive check fires before TOTP validation.
	let otp = setup.compute_totp(&secret);
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ActivateMfaRequest>::builder()
				.headers(ActivateMfaRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ActivateMfaRequest { otp })
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"second activate_mfa call should fail with MfaAlreadyActive"
	);
}

#[tokio::test]
async fn deactivate_mfa_when_inactive() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	// No MFA activated; the handler short-circuits with MfaAlreadyInactive
	// regardless of the OTP supplied.
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeactivateMfaRequest>::builder()
				.headers(DeactivateMfaRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(DeactivateMfaRequest {
					otp: "123456".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"deactivate when MFA is inactive should fail"
	);
}

#[tokio::test]
async fn get_mfa_secret_regenerates() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let key = format!("mfa:{}", user.user_id);

	_ = setup
		.make_web_dashboard_call(
			ApiRequest::<GetMfaSecretRequest>::builder()
				.headers(GetMfaSecretRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetMfaSecretResponse>>();
	let first = setup
		.get_redis_value(&key)
		.await
		.expect("secret missing after first call");

	_ = setup
		.make_web_dashboard_call(
			ApiRequest::<GetMfaSecretRequest>::builder()
				.headers(GetMfaSecretRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetMfaSecretResponse>>();
	let second = setup
		.get_redis_value(&key)
		.await
		.expect("secret missing after second call");

	assert_ne!(
		first, second,
		"two consecutive get_mfa_secret calls should yield different secrets"
	);
}
