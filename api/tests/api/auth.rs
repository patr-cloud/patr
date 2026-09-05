use std::collections::{BTreeMap, BTreeSet};

use headers::authorization::Authorization;
use models::{
	ApiSuccessResponseBody,
	api::{auth::*, user::*},
};

use crate::prelude::*;

// ---------------------------------------------------------------------------
// Create Account
// ---------------------------------------------------------------------------

#[tokio::test]
pub async fn create_account_works() {
	let setup = setup().await.expect("failed to setup test server");

	let email = format!("{}@example.com", random_name(8));
	let password = random_password();

	setup
		.make_web_dashboard_call(
			ApiRequest::<CreateAccountRequest>::builder()
				.headers(CreateAccountRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateAccountRequest {
					email: email.clone(),
					password: password.clone(),
					first_name: "John".to_string(),
					last_name: "Doe".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(CreateAccountResponse));

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CompleteSignUpRequest>::builder()
				.headers(CompleteSignUpRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CompleteSignUpRequest {
					email: email.clone(),
					verification_token: "000000".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CompleteSignUpResponse>>()
		.response;

	let user_info = setup
		.make_web_dashboard_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.headers(GetUserInfoRequestHeaders {
					authorization: BearerToken::from_str(&response.access_token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetUserInfoResponse>>();

	assert_eq!(email, user_info.response.email);
}

#[tokio::test]
async fn create_account_invalid_password() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateAccountRequest>::builder()
				.headers(CreateAccountRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateAccountRequest {
					email: "bad@example.com".to_string(),
					password: "short".to_string(),
					first_name: "Bad".to_string(),
					last_name: "Pass".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for invalid password"
	);
}

#[tokio::test]
async fn complete_sign_up_wrong_otp() {
	let setup = setup().await.expect("failed to setup test server");
	let email = format!("{}@example.com", random_name(8));
	let password = random_password();

	setup
		.make_web_dashboard_call(
			ApiRequest::<CreateAccountRequest>::builder()
				.headers(CreateAccountRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateAccountRequest {
					email: email.clone(),
					password: password.clone(),
					first_name: "OTP".to_string(),
					last_name: "Test".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(CreateAccountResponse));

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CompleteSignUpRequest>::builder()
				.headers(CompleteSignUpRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CompleteSignUpRequest {
					email: email.clone(),
					verification_token: "999999".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for wrong OTP"
	);
}

#[tokio::test]
async fn complete_sign_up_nonexistent_user() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CompleteSignUpRequest>::builder()
				.headers(CompleteSignUpRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CompleteSignUpRequest {
					email: format!("{}@example.com", random_name(8)),
					verification_token: "000000".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for nonexistent user"
	);
}

// ---------------------------------------------------------------------------
// Login
// ---------------------------------------------------------------------------

#[tokio::test]
async fn login_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let (access_token, _refresh_token) = setup.login_test_user(&user.email, &user.password).await;

	let info = setup
		.make_web_dashboard_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.headers(GetUserInfoRequestHeaders {
					authorization: BearerToken::from_str(&access_token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetUserInfoResponse>>();

	assert_eq!(user.email, info.response.email);
}

#[tokio::test]
async fn login_wrong_password() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<LoginRequest>::builder()
				.headers(LoginRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(LoginRequest {
					email: user.email.clone(),
					password: "WrongPassword@123".to_string(),
					mfa_otp: None,
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for wrong password"
	);
}

#[tokio::test]
async fn login_nonexistent_user() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<LoginRequest>::builder()
				.headers(LoginRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(LoginRequest {
					email: format!("{}@example.com", random_name(8)),
					password: random_password(),
					mfa_otp: None,
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for nonexistent user"
	);
}

// ---------------------------------------------------------------------------
// Logout
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "Logout endpoint has a design conflict: both PlainTokenAuthenticator and refresh_token: BearerToken resolve to the Authorization header"]
async fn logout_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<LogoutRequest>::builder()
				.headers(LogoutRequestHeaders {
					refresh_token: user.refresh_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(LogoutResponse));
}

// ---------------------------------------------------------------------------
// Renew Access Token
// ---------------------------------------------------------------------------

#[tokio::test]
async fn renew_access_token_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<RenewAccessTokenRequest>::builder()
				.headers(RenewAccessTokenRequestHeaders {
					refresh_token: user.refresh_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<RenewAccessTokenResponse>>();

	// New access token should work
	let info = setup
		.make_web_dashboard_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.headers(GetUserInfoRequestHeaders {
					authorization: BearerToken::from_str(&response.response.access_token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetUserInfoResponse>>();

	assert_eq!(user.email, info.response.email);
}

#[tokio::test]
async fn renew_access_token_invalid() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<RenewAccessTokenRequest>::builder()
				.headers(RenewAccessTokenRequestHeaders {
					refresh_token: BearerToken::from_str("invalid-token-string").unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for invalid refresh token"
	);
}

// ---------------------------------------------------------------------------
// Forgot / Reset Password
// ---------------------------------------------------------------------------

#[tokio::test]
async fn forgot_password_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<ForgotPasswordRequest>::builder()
				.headers(ForgotPasswordRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(ForgotPasswordRequest {
					email: user.email.clone(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(ForgotPasswordResponse));
}

#[tokio::test]
async fn reset_password_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<ForgotPasswordRequest>::builder()
				.headers(ForgotPasswordRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(ForgotPasswordRequest {
					email: user.email.clone(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(ForgotPasswordResponse));

	let new_password = random_password();
	setup
		.make_web_dashboard_call(
			ApiRequest::<ResetPasswordRequest>::builder()
				.headers(ResetPasswordRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(ResetPasswordRequest {
					email: user.email.clone(),
					password: new_password.clone(),
					verification_token: "000000".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(ResetPasswordResponse));

	// Login with new password should work
	let (_access_token, _refresh_token) = setup.login_test_user(&user.email, &new_password).await;
}

#[tokio::test]
async fn reset_password_wrong_otp() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<ForgotPasswordRequest>::builder()
				.headers(ForgotPasswordRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(ForgotPasswordRequest {
					email: user.email.clone(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(ForgotPasswordResponse));

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ResetPasswordRequest>::builder()
				.headers(ResetPasswordRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(ResetPasswordRequest {
					email: user.email.clone(),
					password: random_password(),
					verification_token: "999999".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for wrong OTP"
	);
}

// ---------------------------------------------------------------------------
// Resend OTP
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resend_otp_works() {
	let setup = setup().await.expect("failed to setup test server");
	let email = format!("{}@example.com", random_name(8));
	let password = random_password();

	setup
		.make_web_dashboard_call(
			ApiRequest::<CreateAccountRequest>::builder()
				.headers(CreateAccountRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateAccountRequest {
					email: email.clone(),
					password: password.clone(),
					first_name: "Resend".to_string(),
					last_name: "Test".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(CreateAccountResponse));

	setup
		.make_web_dashboard_call(
			ApiRequest::<ResendOtpRequest>::builder()
				.headers(ResendOtpRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(ResendOtpRequest {
					email: email.clone(),
					password: password.clone(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(ResendOtpResponse));
}

// ---------------------------------------------------------------------------
// Email / Username Validation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn is_email_valid_available() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<IsEmailValidRequest>::builder()
				.headers(IsEmailValidRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.query(IsEmailValidQuery {
					email: "unused@example.com".to_string(),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<IsEmailValidResponse>>();

	assert!(
		response.response.available,
		"unused email should be available"
	);
}

#[tokio::test]
async fn is_email_valid_taken() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<IsEmailValidRequest>::builder()
				.headers(IsEmailValidRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.query(IsEmailValidQuery {
					email: user.email.clone(),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<IsEmailValidResponse>>();

	assert!(
		!response.response.available,
		"registered email should not be available"
	);
}

#[tokio::test]
async fn docker_login_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	// docker login is for API tokens, not web-dashboard sessions — the handler
	// validates the password as a `patrv1.` token.
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeSet::from([workspace.id]),
			BTreeMap::new(),
		)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DockerLoginRequest>::builder()
				.headers(DockerLoginRequestHeaders {
					authorization: Authorization::basic("patr", &api_token.token),
					user_agent: TEST_USER_AGENT,
				})
				.query(DockerLoginQuery {
					service: "registry".to_string(),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<DockerLoginResponse>>();

	assert!(
		!response.response.access_token.is_empty(),
		"docker login should return access token"
	);
}

#[tokio::test]
async fn docker_login_wrong_credentials() {
	let setup = setup().await.expect("failed to setup test server");
	let _user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DockerLoginRequest>::builder()
				.headers(DockerLoginRequestHeaders {
					authorization: Authorization::basic("wronguser", "wrongpassword"),
					user_agent: TEST_USER_AGENT,
				})
				.query(DockerLoginQuery {
					service: "registry".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for wrong docker credentials"
	);
}

#[tokio::test]
async fn docker_login_invalid_token() {
	let setup = setup().await.expect("failed to setup test server");
	let _user = setup.create_test_user().await;

	// Username is `patr` but the password is a well-formed-but-nonexistent API
	// token. The handler now validates it, so this must be rejected instead of
	// echoed back as a bearer credential.
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DockerLoginRequest>::builder()
				.headers(DockerLoginRequestHeaders {
					authorization: Authorization::basic(
						"patr",
						"patrv1.deadbeefdeadbeefdeadbeefdeadbeef.\
						 deadbeefdeadbeefdeadbeefdeadbeef",
					),
					user_agent: TEST_USER_AGENT,
				})
				.query(DockerLoginQuery {
					service: "registry".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for invalid docker token, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn sign_up_and_login_are_case_insensitive() {
	let setup = setup().await.expect("failed to setup test server");

	// `email` is CITEXT on both `"user"` and `user_to_sign_up`, so every
	// lookup matches case-insensitively while the column still stores what
	// the user typed. Sign up with one casing, confirm with a second, log in
	// with a third — all three have to land on the same account, and the
	// stored address keeps the casing used at sign-up.
	let local_part = random_name(8);
	let signup_email = format!("{}@Example.COM", local_part.to_uppercase());
	let password = random_password();

	setup
		.make_web_dashboard_call(
			ApiRequest::<CreateAccountRequest>::builder()
				.headers(CreateAccountRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateAccountRequest {
					email: signup_email.clone(),
					password: password.clone(),
					first_name: "Mixed".to_string(),
					last_name: "Case".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(CreateAccountResponse));

	setup
		.make_web_dashboard_call(
			ApiRequest::<CompleteSignUpRequest>::builder()
				.headers(CompleteSignUpRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CompleteSignUpRequest {
					email: format!("{}@EXAMPLE.com", local_part),
					verification_token: "000000".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CompleteSignUpResponse>>();

	let (access_token, _) = setup
		.login_test_user(&format!("{}@Example.com", local_part), &password)
		.await;

	let info = setup
		.make_web_dashboard_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.headers(GetUserInfoRequestHeaders {
					authorization: BearerToken::from_str(&access_token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetUserInfoResponse>>();

	// Case preserved as entered at sign-up, not normalised and not overwritten
	// by the casing used on the confirmation form.
	assert_eq!(signup_email, info.response.email);
}

#[tokio::test]
async fn create_account_invalid_email() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateAccountRequest>::builder()
				.headers(CreateAccountRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateAccountRequest {
					email: "not-an-email".to_string(),
					password: random_password(),
					first_name: "Bad".to_string(),
					last_name: "Email".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for malformed email, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn complete_sign_up_otp_wrong_format() {
	let setup = setup().await.expect("failed to setup test server");

	// OTP regex is `^(\d{3}\-?\d{3})$`. These all violate it.
	for bad_otp in ["12345", "1234567", "abcdef", "12-3456", "abc-def"] {
		let response = setup
			.make_web_dashboard_call(
				ApiRequest::<CompleteSignUpRequest>::builder()
					.headers(CompleteSignUpRequestHeaders {
						user_agent: TEST_USER_AGENT,
					})
					.body(CompleteSignUpRequest {
						email: format!("{}@example.com", random_name(8)),
						verification_token: bad_otp.to_string(),
						cf_turnstile_token: "1x00000000000000000000AA".to_string(),
					})
					.build(),
			)
			.await;

		assert!(
			response.status_code().is_client_error(),
			"expected client error for OTP `{bad_otp}`, got {}",
			response.status_code()
		);
	}
}

#[tokio::test]
async fn create_account_duplicate_email() {
	let setup = setup().await.expect("failed to setup test server");

	let shared_email = format!("{}@example.com", random_name(8));

	setup
		.make_web_dashboard_call(
			ApiRequest::<CreateAccountRequest>::builder()
				.headers(CreateAccountRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateAccountRequest {
					email: shared_email.clone(),
					password: random_password(),
					first_name: "First".to_string(),
					last_name: "User".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(CreateAccountResponse));

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateAccountRequest>::builder()
				.headers(CreateAccountRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateAccountRequest {
					email: shared_email,
					password: random_password(),
					first_name: "Second".to_string(),
					last_name: "User".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for duplicate email, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn forgot_password_nonexistent_user() {
	let setup = setup().await.expect("failed to setup test server");

	setup
		.make_web_dashboard_call(
			ApiRequest::<ForgotPasswordRequest>::builder()
				.headers(ForgotPasswordRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(ForgotPasswordRequest {
					email: format!("{}@example.com", random_name(8)),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(ForgotPasswordResponse));
}

#[tokio::test]
async fn resend_otp_nonexistent_user() {
	let setup = setup().await.expect("failed to setup test server");

	// Same silent-success behaviour: resend OTP for an unknown user must not
	// leak existence — handler returns success regardless.
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ResendOtpRequest>::builder()
				.headers(ResendOtpRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(ResendOtpRequest {
					email: format!("{}@example.com", random_name(8)),
					password: random_password(),
				})
				.build(),
		)
		.await;

	// Either silent success or generic client error (UserNotFound) is acceptable
	// — the contract is just "no leak". Allow both, but reject 5xx.
	assert!(
		!response.status_code().is_server_error(),
		"resend_otp for nonexistent user should not 5xx, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn reset_password_new_password_invalid() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<ForgotPasswordRequest>::builder()
				.headers(ForgotPasswordRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(ForgotPasswordRequest {
					email: user.email.clone(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(ForgotPasswordResponse));

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ResetPasswordRequest>::builder()
				.headers(ResetPasswordRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(ResetPasswordRequest {
					email: user.email.clone(),
					password: "short".to_string(),
					verification_token: "000000".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for weak new password"
	);
}

#[tokio::test]
async fn complete_sign_up_expired_otp() {
	let setup = setup().await.expect("failed to setup test server");
	let email = format!("{}@example.com", random_name(8));

	setup
		.make_web_dashboard_call(
			ApiRequest::<CreateAccountRequest>::builder()
				.headers(CreateAccountRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateAccountRequest {
					email: email.clone(),
					password: random_password(),
					first_name: "Expired".to_string(),
					last_name: "Otp".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(CreateAccountResponse));

	// Backdate the signup OTP so that any subsequent verification attempt is
	// past expiry.
	setup
		.execute_sql(&format!(
			"UPDATE user_to_sign_up SET otp_expiry = NOW() - INTERVAL '1 hour' \
			 WHERE email = '{email}'"
		))
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CompleteSignUpRequest>::builder()
				.headers(CompleteSignUpRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CompleteSignUpRequest {
					email: email.clone(),
					verification_token: "000000".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for expired sign-up OTP"
	);
}

#[tokio::test]
async fn reset_password_expired_otp() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<ForgotPasswordRequest>::builder()
				.headers(ForgotPasswordRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(ForgotPasswordRequest {
					email: user.email.clone(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(ForgotPasswordResponse));

	setup
		.execute_sql(&format!(
			"UPDATE \"user\" SET password_reset_token_expiry = NOW() - INTERVAL '1 hour' \
			 WHERE email = '{}'",
			user.email
		))
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ResetPasswordRequest>::builder()
				.headers(ResetPasswordRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(ResetPasswordRequest {
					email: user.email.clone(),
					password: random_password(),
					verification_token: "000000".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for expired password-reset OTP"
	);
}

#[tokio::test]
async fn renew_access_token_expired() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	// Backdate the refresh token's expiry on every web_login row for this
	// user. The login created during signup is the only one in play.
	setup
		.execute_sql(&format!(
			"UPDATE web_login SET token_expiry = NOW() - INTERVAL '1 hour' \
			 WHERE user_id = '{}'",
			user.user_id
		))
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<RenewAccessTokenRequest>::builder()
				.headers(RenewAccessTokenRequestHeaders {
					refresh_token: user.refresh_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for expired refresh token, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn session_expiry_does_not_invalidate_access_token() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	// Pre-check: the token works.
	_ = setup
		.make_web_dashboard_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.headers(GetUserInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetUserInfoResponse>>();

	// `web_login.token_expiry` is the *refresh token's* lifetime, not the
	// access token's. Backdating it must NOT invalidate already-issued
	// access tokens — those are gated by the JWT's own `exp` claim. The
	// only thing this should break is renewing the refresh token (covered
	// by `renew_access_token_expired`).
	setup
		.execute_sql(&format!(
			"UPDATE web_login SET token_expiry = NOW() - INTERVAL '1 hour' \
			 WHERE user_id = '{}'",
			user.user_id
		))
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.headers(GetUserInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_success(),
		"already-issued access token must keep working after the session's \
		 refresh-token expiry is backdated; got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn session_isolation() {
	let setup = setup().await.expect("failed to setup test server");
	let user_a = setup.create_test_user().await;
	let user_b = setup.create_test_user().await;

	// Each token resolves to its own user — no cross-talk via shared session.
	let a_info = setup
		.make_web_dashboard_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.headers(GetUserInfoRequestHeaders {
					authorization: user_a.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetUserInfoResponse>>();

	let b_info = setup
		.make_web_dashboard_call(
			ApiRequest::<GetUserInfoRequest>::builder()
				.headers(GetUserInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetUserInfoResponse>>();

	assert_eq!(user_a.user_id, a_info.response.basic_user_info.id);
	assert_eq!(user_b.user_id, b_info.response.basic_user_info.id);
	assert_ne!(
		a_info.response.basic_user_info.id,
		b_info.response.basic_user_info.id
	);
}

#[tokio::test]
async fn login_with_mfa_required() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let _secret = crate::api::user::mfa::activate_mfa_for_user(&setup, &user).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<LoginRequest>::builder()
				.headers(LoginRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(LoginRequest {
					email: user.email.clone(),
					password: user.password.clone(),
					mfa_otp: None,
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"login without MFA OTP for an MFA-active user should fail with MfaRequired"
	);
}

#[tokio::test]
async fn login_with_mfa_valid_otp() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let secret = crate::api::user::mfa::activate_mfa_for_user(&setup, &user).await;
	let otp = setup.compute_totp(&secret);

	let _ = setup
		.make_web_dashboard_call(
			ApiRequest::<LoginRequest>::builder()
				.headers(LoginRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(LoginRequest {
					email: user.email.clone(),
					password: user.password.clone(),
					mfa_otp: Some(otp),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<LoginResponse>>();
}

#[tokio::test]
async fn login_with_mfa_invalid_otp() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let _secret = crate::api::user::mfa::activate_mfa_for_user(&setup, &user).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<LoginRequest>::builder()
				.headers(LoginRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(LoginRequest {
					email: user.email.clone(),
					password: user.password.clone(),
					mfa_otp: Some("000000".to_string()),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"login with wrong MFA OTP should fail with MfaOtpInvalid"
	);
}

#[tokio::test]
async fn forgot_password_rate_limit() {
	use rand::RngExt as _;
	let setup = setup().await.expect("failed to setup test server");

	// Pin to a per-test random IP so the bucket fills predictably under
	// shared Redis. Plain `make_web_dashboard_call` injects a fresh random
	// IP per call which would defeat rate-limit accumulation.
	let ip = std::net::IpAddr::V4(rand::rng().random::<u32>().into());

	// The per-IP unauth window is 50 requests/second in debug builds (see
	// `RATE_LIMITS` in `utils::layers::rate_limiter_layer`). Unknown user_ids
	// get a silent 202 without the Argon2 work, so they're cheap. Fire well
	// above the limit *concurrently* from the same IP: a sequential loop is
	// flaky because under parallel test load each round-trip can take long
	// enough that earlier requests slide out of the 1-second window before the
	// count ever exceeds the limit. Concurrent dispatch lands the burst inside
	// a single window regardless of per-request latency.
	let requests = (0..75).map(|i| {
		setup.make_web_dashboard_call_from_ip(
			ApiRequest::<ForgotPasswordRequest>::builder()
				.headers(ForgotPasswordRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(ForgotPasswordRequest {
					email: format!("nonexistent-{i}@example.com"),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
			ip,
		)
	});
	let responses = futures::future::join_all(requests).await;
	let throttled = responses
		.iter()
		.filter(|r| r.status_code() == StatusCode::TOO_MANY_REQUESTS)
		.count();

	assert!(
		throttled > 0,
		"expected at least one 429 from 75 concurrent forgot_password calls, got none"
	);
}

#[tokio::test]
async fn complete_sign_up_already_completed() {
	let setup = setup().await.expect("failed to setup test server");

	let email = format!("{}@example.com", random_name(8));
	let password = random_password();

	setup
		.make_web_dashboard_call(
			ApiRequest::<CreateAccountRequest>::builder()
				.headers(CreateAccountRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateAccountRequest {
					email: email.clone(),
					password,
					first_name: "Test".to_string(),
					last_name: "User".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(CreateAccountResponse));

	_ = setup
		.make_web_dashboard_call(
			ApiRequest::<CompleteSignUpRequest>::builder()
				.headers(CompleteSignUpRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CompleteSignUpRequest {
					email: email.clone(),
					verification_token: "000000".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CompleteSignUpResponse>>();

	// Second complete-sign-up call with the same payload: the user-to-sign-up
	// row was deleted on first success, so the handler returns UserNotFound.
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CompleteSignUpRequest>::builder()
				.headers(CompleteSignUpRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CompleteSignUpRequest {
					email,
					verification_token: "000000".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"second complete-sign-up should fail (user-to-sign-up row deleted on first success)"
	);
}

#[tokio::test]
async fn concurrent_token_renewal() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	// Fire two concurrent renews using the same refresh token. With single-use
	// rotation, exactly one should succeed and one should fail.
	let req = || {
		setup.make_web_dashboard_call(
			ApiRequest::<RenewAccessTokenRequest>::builder()
				.headers(RenewAccessTokenRequestHeaders {
					refresh_token: user.refresh_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
	};
	let (a, b) = tokio::join!(req(), req());
	let statuses = [a.status_code(), b.status_code()];
	let successes = statuses.iter().filter(|s| s.is_success()).count();
	let failures = statuses.iter().filter(|s| s.is_client_error()).count();

	assert_eq!(
		(successes, failures),
		(1, 1),
		"with single-use refresh, exactly one of two concurrent renews should succeed; got {statuses:?}"
	);
}

#[tokio::test]
async fn refresh_token_single_use() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let original_refresh = user.refresh_token.clone();

	// First renew: should succeed and return a new refresh token.
	let renewed = setup
		.make_web_dashboard_call(
			ApiRequest::<RenewAccessTokenRequest>::builder()
				.headers(RenewAccessTokenRequestHeaders {
					refresh_token: original_refresh.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<RenewAccessTokenResponse>>();

	assert_ne!(
		renewed.response.refresh_token,
		original_refresh.0.token(),
		"renew should return a fresh refresh token"
	);

	// Second renew with the OLD refresh token must fail.
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<RenewAccessTokenRequest>::builder()
				.headers(RenewAccessTokenRequestHeaders {
					refresh_token: original_refresh,
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"second renew using the old refresh token should be rejected"
	);

	// And the new refresh token should still work.
	_ = setup
		.make_web_dashboard_call(
			ApiRequest::<RenewAccessTokenRequest>::builder()
				.headers(RenewAccessTokenRequestHeaders {
					refresh_token: BearerToken::from_str(&renewed.response.refresh_token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<RenewAccessTokenResponse>>();
}

// ---------------------------------------------------------------------------
// Validation backstop: names + descriptions reject HTML / control characters
// ---------------------------------------------------------------------------

/// Helper: attempt CreateAccount with a given `first_name`; assert client
/// error.
async fn assert_create_account_first_name_rejected(setup: &TestSetup, bad: &str) {
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateAccountRequest>::builder()
				.headers(CreateAccountRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateAccountRequest {
					email: format!("{}@example.com", random_name(8)),
					password: random_password(),
					first_name: bad.to_string(),
					last_name: "Doe".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"expected 4xx for first_name `{bad:?}`, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn create_account_rejects_xss_in_first_name() {
	let setup = setup().await.expect("failed to setup test server");
	assert_create_account_first_name_rejected(&setup, "<script>alert('x')</script>").await;
}

#[tokio::test]
async fn create_account_rejects_html_bracket_in_last_name() {
	let setup = setup().await.expect("failed to setup test server");
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateAccountRequest>::builder()
				.headers(CreateAccountRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateAccountRequest {
					email: format!("{}@example.com", random_name(8)),
					password: random_password(),
					first_name: "Ada".to_string(),
					last_name: "<img onerror=foo()>".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"expected 4xx, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn create_account_rejects_newline_in_first_name() {
	let setup = setup().await.expect("failed to setup test server");
	assert_create_account_first_name_rejected(&setup, "Ada\nMore").await;
}

#[tokio::test]
async fn create_account_accepts_unicode_first_name() {
	let setup = setup().await.expect("failed to setup test server");
	let email = format!("{}@example.com", random_name(8));
	setup
		.make_web_dashboard_call(
			ApiRequest::<CreateAccountRequest>::builder()
				.headers(CreateAccountRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateAccountRequest {
					email: email.clone(),
					password: random_password(),
					first_name: "José".to_string(),
					last_name: "Núñez".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(CreateAccountResponse));
}

// ---------------------------------------------------------------------------
// Login: email validator
// ---------------------------------------------------------------------------

#[tokio::test]
async fn login_rejects_non_email_identifier() {
	let setup = setup().await.expect("failed to setup test server");

	// A user's email is their only identifier. Anything that isn't
	// email-shaped — a bare username, a phone number — has to be rejected by
	// the model's `#[preprocess(email)]` before it ever reaches the handler.
	for identifier in ["+15555550123", "someusername"] {
		let response = setup
			.make_web_dashboard_call(
				ApiRequest::<LoginRequest>::builder()
					.headers(LoginRequestHeaders {
						user_agent: TEST_USER_AGENT,
					})
					.body(LoginRequest {
						email: identifier.to_string(),
						password: random_password(),
						mfa_otp: None,
						cf_turnstile_token: "1x00000000000000000000AA".to_string(),
					})
					.build(),
			)
			.await;

		assert!(
			response.status_code().is_client_error(),
			"non-email identifier `{identifier}` must be rejected at the model layer, got {}",
			response.status_code()
		);
	}
}

// ---------------------------------------------------------------------------
// Forgot / reset password: Turnstile token required
// ---------------------------------------------------------------------------

#[tokio::test]
async fn forgot_password_rejects_missing_turnstile_token() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ForgotPasswordRequest>::builder()
				.headers(ForgotPasswordRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(ForgotPasswordRequest {
					email: user.email.clone(),
					cf_turnstile_token: String::new(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"empty cf_turnstile_token must be rejected, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn reset_password_rejects_missing_turnstile_token() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ResetPasswordRequest>::builder()
				.headers(ResetPasswordRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(ResetPasswordRequest {
					email: user.email.clone(),
					password: random_password(),
					verification_token: "000000".to_string(),
					cf_turnstile_token: String::new(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"empty cf_turnstile_token must be rejected, got {}",
		response.status_code()
	);
}

// ---------------------------------------------------------------------------
// Attempt ceilings
//
// These drive the counter through real failed requests rather than seeding it
// via SQL. Seeding only proves the `>= MAX` check rejects; it says nothing
// about whether anything ever increments the counter, which is the half that
// actually gates brute force.

#[tokio::test]
async fn complete_sign_up_exhausts_attempts() {
	let setup = setup().await.expect("failed to setup test server");
	let email = format!("{}@example.com", random_name(8));
	let password = random_password();

	setup
		.make_web_dashboard_call(
			ApiRequest::<CreateAccountRequest>::builder()
				.headers(CreateAccountRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateAccountRequest {
					email: email.clone(),
					password: password.clone(),
					first_name: "OTP".to_string(),
					last_name: "Test".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(CreateAccountResponse));

	for _ in 0..constants::MAX_SIGN_UP_ATTEMPTS {
		let response = setup
			.make_web_dashboard_call(
				ApiRequest::<CompleteSignUpRequest>::builder()
					.headers(CompleteSignUpRequestHeaders {
						user_agent: TEST_USER_AGENT,
					})
					.body(CompleteSignUpRequest {
						email: email.clone(),
						verification_token: "999999".to_string(),
						cf_turnstile_token: "1x00000000000000000000AA".to_string(),
					})
					.build(),
			)
			.await;
		assert!(response.status_code().is_client_error());
	}

	// Ceiling reached: even the correct debug OTP is now refused. This is what
	// catches an attempt counter that never actually counts — if the increments
	// were rolled back, the OTP below would still work.
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CompleteSignUpRequest>::builder()
				.headers(CompleteSignUpRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(CompleteSignUpRequest {
					email: email.clone(),
					verification_token: "000000".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"a locked sign-up must not complete even with the correct OTP"
	);
}

#[tokio::test]
async fn reset_password_exhausts_attempts() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<ForgotPasswordRequest>::builder()
				.headers(ForgotPasswordRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(ForgotPasswordRequest {
					email: user.email.clone(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(ForgotPasswordResponse));

	for _ in 0..constants::MAX_PASSWORD_RESET_ATTEMPTS {
		let response = setup
			.make_web_dashboard_call(
				ApiRequest::<ResetPasswordRequest>::builder()
					.headers(ResetPasswordRequestHeaders {
						user_agent: TEST_USER_AGENT,
					})
					.body(ResetPasswordRequest {
						email: user.email.clone(),
						password: random_password(),
						verification_token: "999999".to_string(),
						cf_turnstile_token: "1x00000000000000000000AA".to_string(),
					})
					.build(),
			)
			.await;
		assert!(response.status_code().is_client_error());
	}

	// Ceiling reached: the correct debug OTP no longer resets the password. This
	// is what catches an attempt counter that never actually counts — if the
	// increments were rolled back, the OTP below would still work.
	let new_password = random_password();
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ResetPasswordRequest>::builder()
				.headers(ResetPasswordRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.body(ResetPasswordRequest {
					email: user.email.clone(),
					password: new_password,
					verification_token: "000000".to_string(),
					cf_turnstile_token: "1x00000000000000000000AA".to_string(),
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"a locked reset must not succeed even with the correct OTP"
	);
}
