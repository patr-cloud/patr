use http::header;
use models::{
	ApiSuccessResponseBody,
	api::{ApiEndpoint, auth::*, user::*},
};
use rand::{Rng as _, distributions::Alphanumeric};

use crate::prelude::*;

#[tokio::test]
pub async fn create_account_works() {
	let setup = setup().await.expect("failed to setup test server");

	let username = rand::thread_rng()
		.sample_iter(Alphanumeric)
		.map(char::from)
		.take(8)
		.collect::<String>();

	let password = format!(
		"{}@",
		rand::thread_rng()
			.sample_iter(Alphanumeric)
			.map(char::from)
			.take(32)
			.collect::<String>()
	);

	setup
		.server
		.method(CreateAccountRequest::METHOD, &CreateAccountPath.to_string())
		.add_header(header::USER_AGENT, "cargo-test")
		.json(&CreateAccountRequest {
			username: username.clone(),
			password: password.clone(),
			first_name: "John".to_string(),
			last_name: "Doe".to_string(),
			recovery_method: RecoveryMethod::Email {
				recovery_email: "hello@example.com".to_string(),
			},
			cf_turnstile_token: "1x00000000000000000000AA".to_string(),
		})
		.await
		.assert_json(&ApiSuccessResponseBody::new(CreateAccountResponse));

	let response = setup
		.server
		.method(
			CompleteSignUpRequest::METHOD,
			&CompleteSignUpPath.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.json(&CompleteSignUpRequest {
			username: username.clone(),
			verification_token: "000000".to_string(),
			cf_turnstile_token: "1x00000000000000000000AA".to_string(),
		})
		.await
		.json::<ApiSuccessResponseBody<CompleteSignUpResponse>>()
		.response;

	let user_info = setup
		.server
		.method(GetUserInfoRequest::METHOD, &GetUserInfoPath.to_string())
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(response.access_token)
		.await
		.json::<ApiSuccessResponseBody<GetUserInfoResponse>>();

	assert_eq!(username, user_info.response.basic_user_info.username);
}
