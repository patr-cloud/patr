use http::header;
use models::{
	ApiSuccessResponseBody,
	api::{ApiEndpoint, user::*},
	utils::Uuid,
};

use crate::prelude::*;

#[tokio::test]
async fn create_api_token_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;

	let api_token = create_test_api_token(&setup, &user.access_token).await;
	assert!(!api_token.token.is_empty(), "token should not be empty");
}

#[tokio::test]
async fn list_api_tokens_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;

	let _t1 = create_test_api_token(&setup, &user.access_token).await;
	let _t2 = create_test_api_token(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			ListApiTokensRequest::METHOD,
			&ListApiTokensPath.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListApiTokensResponse>>();

	assert!(
		response.response.tokens.len() >= 2,
		"should have at least 2 tokens"
	);
}

#[tokio::test]
async fn get_api_token_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let api_token = create_test_api_token(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			GetApiTokenInfoRequest::METHOD,
			&GetApiTokenInfoPath {
				token_id: api_token.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<GetApiTokenInfoResponse>>();

	assert_eq!(api_token.name, response.response.token.name);
}

#[tokio::test]
async fn update_api_token_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let api_token = create_test_api_token(&setup, &user.access_token).await;
	let new_name = random_name(8);

	setup
		.server
		.method(
			UpdateApiTokenRequest::METHOD,
			&UpdateApiTokenPath {
				token_id: api_token.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.json(&UpdateApiTokenRequest {
			name: Some(new_name.clone()),
			permissions: None,
			token_nbf: None,
			token_exp: None,
			allowed_ips: None,
		})
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateApiTokenResponse));

	// Verify the update
	let response = setup
		.server
		.method(
			GetApiTokenInfoRequest::METHOD,
			&GetApiTokenInfoPath {
				token_id: api_token.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<GetApiTokenInfoResponse>>();

	assert_eq!(new_name, response.response.token.name);
}

#[tokio::test]
async fn revoke_api_token_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let api_token = create_test_api_token(&setup, &user.access_token).await;

	setup
		.server
		.method(
			RevokeApiTokenRequest::METHOD,
			&RevokeApiTokenPath {
				token_id: api_token.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.assert_json(&ApiSuccessResponseBody::new(RevokeApiTokenResponse));

	// Verify it's gone
	let response = setup
		.server
		.method(
			GetApiTokenInfoRequest::METHOD,
			&GetApiTokenInfoPath {
				token_id: api_token.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"revoked token should not be found"
	);
}

#[tokio::test]
async fn regenerate_api_token_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let api_token = create_test_api_token(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			RegenerateApiTokenRequest::METHOD,
			&RegenerateApiTokenPath {
				token_id: api_token.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<RegenerateApiTokenResponse>>();

	assert_ne!(
		api_token.token, response.response.token,
		"regenerated token should be different"
	);
}

#[tokio::test]
async fn get_api_token_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;

	let response = setup
		.server
		.method(
			GetApiTokenInfoRequest::METHOD,
			&GetApiTokenInfoPath {
				token_id: Uuid::nil(),
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for nonexistent token"
	);
}

#[tokio::test]
async fn api_token_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");

	let response = setup
		.server
		.method(
			ListApiTokensRequest::METHOD,
			&ListApiTokensPath.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error without auth token"
	);
}
