//! When a token is accepted at all: the nbf/exp window, malformed and unknown
//! tokens, and the allowed-IP list.

use std::{collections::BTreeMap, net::IpAddr, str::FromStr};

use ipnetwork::IpNetwork;
use models::{ApiSuccessResponseBody, api::user::*, rbac::WorkspacePermission, utils::Uuid};

use super::{call_with_token, mint_token_raw};
use crate::prelude::*;

/// A token used after its `token_exp` is rejected at auth time (401).
#[tokio::test]
async fn api_token_expired_is_rejected() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let token = mint_token_raw(
		&setup,
		&user.access_token,
		BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		None,
		Some(time::OffsetDateTime::now_utc() - time::Duration::minutes(1)),
		None,
	)
	.await
	.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
	.response
	.token;

	assert_eq!(
		401,
		call_with_token(&setup, &token).await.status_code().as_u16(),
		"an expired token should be rejected with 401"
	);
}

/// A token used before its `token_nbf` is rejected at auth time (401).
#[tokio::test]
async fn api_token_before_nbf_is_rejected() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let token = mint_token_raw(
		&setup,
		&user.access_token,
		BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		Some(time::OffsetDateTime::now_utc() + time::Duration::hours(1)),
		None,
		None,
	)
	.await
	.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
	.response
	.token;

	assert_eq!(
		401,
		call_with_token(&setup, &token).await.status_code().as_u16(),
		"a token used before its NBF should be rejected with 401"
	);
}

/// A token whose NBF is now and EXP is far in the future is accepted.
#[tokio::test]
async fn api_token_valid_nbf_exp_accepted() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let token = mint_token_raw(
		&setup,
		&user.access_token,
		BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		Some(time::OffsetDateTime::now_utc() - time::Duration::minutes(1)),
		Some(time::OffsetDateTime::now_utc() + time::Duration::days(7)),
		None,
	)
	.await
	.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
	.response
	.token;

	assert!(
		call_with_token(&setup, &token)
			.await
			.status_code()
			.is_success(),
		"a token within its NBF..EXP window should be accepted"
	);
}

/// Minting a token with NBF later than EXP is rejected (400).
#[tokio::test]
async fn api_token_nbf_after_exp_rejected_on_create() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let resp = mint_token_raw(
		&setup,
		&user.access_token,
		BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		Some(time::OffsetDateTime::now_utc() + time::Duration::days(7)),
		Some(time::OffsetDateTime::now_utc() + time::Duration::days(1)),
		None,
	)
	.await;
	assert_eq!(
		400,
		resp.status_code().as_u16(),
		"minting a token with NBF > EXP should be 400"
	);
}

/// A PATCH that lands the token in NBF > EXP is rejected (400).
#[tokio::test]
async fn api_token_nbf_after_exp_rejected_on_patch() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let id = mint_token_raw(
		&setup,
		&user.access_token,
		BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		None,
		Some(time::OffsetDateTime::now_utc() + time::Duration::days(1)),
		None,
	)
	.await
	.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
	.response
	.id;

	let resp = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateApiTokenRequest>::builder()
				.path(UpdateApiTokenPath { token_id: id })
				.headers(UpdateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateApiTokenRequest {
					token: UserApiToken {
						name: "patchtoken".to_string(),
						permissions: BTreeMap::from([(
							workspace.id,
							WorkspacePermission::SuperAdmin,
						)]),
						// nbf 7 days out, exp 1 day out (resent) → nbf > exp → 400.
						token_nbf: Some(time::OffsetDateTime::now_utc() + time::Duration::days(7)),
						token_exp: Some(time::OffsetDateTime::now_utc() + time::Duration::days(1)),
						allowed_ips: None,
						created: time::OffsetDateTime::now_utc(),
					},
				})
				.build(),
		)
		.await;
	assert_eq!(
		400,
		resp.status_code().as_u16(),
		"a PATCH landing NBF > EXP should be 400"
	);
}

/// A token created with an empty `allowed_ips` list is callable (empty list is
/// normalized to "no whitelist", not "block all").
#[tokio::test]
async fn api_token_empty_allowed_ips_callable() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let token = mint_token_raw(
		&setup,
		&user.access_token,
		BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		None,
		None,
		Some(vec![]),
	)
	.await
	.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
	.response
	.token;

	assert!(
		call_with_token(&setup, &token)
			.await
			.status_code()
			.is_success(),
		"empty allowed_ips should not block the token"
	);
}

/// A malformed token is rejected with 400.
#[tokio::test]
async fn api_token_malformed_rejected() {
	let setup = setup().await.expect("failed to setup test server");
	assert_eq!(
		400,
		call_with_token(&setup, "patrv1.garbage")
			.await
			.status_code()
			.as_u16(),
		"a malformed token should be 400"
	);
}

/// A well-formed but unknown token is rejected with 401.
#[tokio::test]
async fn api_token_unknown_rejected() {
	let setup = setup().await.expect("failed to setup test server");
	let fake = format!("patrv1.{}.{}", Uuid::nil(), Uuid::nil());
	assert_eq!(
		401,
		call_with_token(&setup, &fake).await.status_code().as_u16(),
		"a well-formed but unknown token should be 401"
	);
}

#[tokio::test]
async fn api_token_with_ip_restriction_allows_listed_ip() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let allowed: IpAddr = "1.2.3.4".parse().unwrap();
	let create = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateApiTokenRequest>::builder()
				.headers(CreateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateApiTokenRequest {
					token: UserApiToken {
						name: random_name(8),
						permissions: BTreeMap::from([(
							workspace.id,
							WorkspacePermission::SuperAdmin,
						)]),
						token_nbf: None,
						token_exp: None,
						allowed_ips: Some(vec![IpNetwork::from(allowed)]),
						created: time::OffsetDateTime::now_utc(),
					},
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
		.response;

	let token_bearer = BearerToken::from_str(&create.token).unwrap();
	let response = setup
		.make_api_call_from_ip(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: token_bearer,
					user_agent: TEST_USER_AGENT,
				})
				.build(),
			allowed,
		)
		.await;

	assert!(
		response.status_code().is_success(),
		"request from allowed IP should succeed, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn api_token_with_ip_restriction_blocks_unlisted_ip() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let allowed: IpAddr = "1.2.3.4".parse().unwrap();
	let blocked: IpAddr = "5.6.7.8".parse().unwrap();
	let create = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateApiTokenRequest>::builder()
				.headers(CreateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateApiTokenRequest {
					token: UserApiToken {
						name: random_name(8),
						permissions: BTreeMap::from([(
							workspace.id,
							WorkspacePermission::SuperAdmin,
						)]),
						token_nbf: None,
						token_exp: None,
						allowed_ips: Some(vec![IpNetwork::from(allowed)]),
						created: time::OffsetDateTime::now_utc(),
					},
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
		.response;

	let token_bearer = BearerToken::from_str(&create.token).unwrap();
	let response = setup
		.make_api_call_from_ip(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: token_bearer,
					user_agent: TEST_USER_AGENT,
				})
				.build(),
			blocked,
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"request from disallowed IP should be rejected, got {}",
		response.status_code()
	);
}
