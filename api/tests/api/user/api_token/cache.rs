//! The auth cache. A used token is served from Redis without touching the
//! database; a changed credential stops working at once regardless. Each test
//! uses the token first so it is cached, then acts and checks the next call.

use std::{collections::BTreeMap, net::IpAddr, str::FromStr};

use ipnetwork::IpNetwork;
use models::{ApiSuccessResponseBody, api::user::*, rbac::WorkspacePermission};

use super::{call_with_token, mint_token_raw};
use crate::prelude::*;

/// Once a token has been used, its next call is served from the cache and
/// never reaches the database: revoking it behind the API's back (so nothing
/// marks the cache stale) doesn't stop it.
#[tokio::test]
async fn used_api_token_is_served_from_cache() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	assert!(
		call_with_token(&setup, &api_token.token)
			.await
			.status_code()
			.is_success(),
		"the first call should populate the cache"
	);

	sqlx::query(&format!(
		"UPDATE user_api_token SET revoked = NOW() WHERE token_id = '{}'",
		api_token.id
	))
	.execute(setup.database())
	.await
	.expect("revoke query");

	assert!(
		call_with_token(&setup, &api_token.token)
			.await
			.status_code()
			.is_success(),
		"the second call should be served from the cache without a database lookup"
	);
}

/// A cached login doesn't vouch for the secret: once a token has been used,
/// the same login ID with a wrong secret is still rejected.
#[tokio::test]
async fn cached_api_token_still_checks_the_secret() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	assert!(
		call_with_token(&setup, &api_token.token)
			.await
			.status_code()
			.is_success(),
		"the first call should populate the cache"
	);

	let forged = format!("patrv1.{}.{}", Uuid::new_v4(), api_token.id);
	assert_eq!(
		401,
		call_with_token(&setup, &forged)
			.await
			.status_code()
			.as_u16(),
		"a wrong secret for a cached login should be rejected with 401"
	);
}

/// A revoked token is rejected on the next call.
#[tokio::test]
async fn revoked_api_token_is_rejected_immediately() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	assert!(
		call_with_token(&setup, &api_token.token)
			.await
			.status_code()
			.is_success(),
		"the token should work before it is revoked"
	);

	setup
		.make_web_dashboard_call(
			ApiRequest::<RevokeApiTokenRequest>::builder()
				.path(RevokeApiTokenPath {
					token_id: api_token.id,
				})
				.headers(RevokeApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(RevokeApiTokenResponse));

	assert_eq!(
		401,
		call_with_token(&setup, &api_token.token)
			.await
			.status_code()
			.as_u16(),
		"a revoked token should be rejected with 401"
	);
}

/// After regeneration the old secret is rejected and the new one works.
#[tokio::test]
async fn regenerated_api_token_old_secret_is_rejected_immediately() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	assert!(
		call_with_token(&setup, &api_token.token)
			.await
			.status_code()
			.is_success(),
		"the token should work before it is regenerated"
	);

	let new_token = setup
		.make_web_dashboard_call(
			ApiRequest::<RegenerateApiTokenRequest>::builder()
				.path(RegenerateApiTokenPath {
					token_id: api_token.id,
				})
				.headers(RegenerateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<RegenerateApiTokenResponse>>()
		.response
		.token;

	assert_eq!(
		401,
		call_with_token(&setup, &api_token.token)
			.await
			.status_code()
			.as_u16(),
		"the old secret should be rejected with 401"
	);
	assert!(
		call_with_token(&setup, &new_token)
			.await
			.status_code()
			.is_success(),
		"the regenerated token should work"
	);
}

/// Adding an IP restriction to a token applies to the next call.
#[tokio::test]
async fn update_api_token_allowed_ips_takes_effect_immediately() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let permissions = BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]);

	let created = mint_token_raw(
		&setup,
		&user.access_token,
		permissions.clone(),
		None,
		None,
		None,
	)
	.await
	.json::<ApiSuccessResponseBody<CreateApiTokenResponse>>()
	.response;
	let token_bearer = BearerToken::from_str(&created.token).unwrap();

	let allowed: IpAddr = "1.2.3.4".parse().unwrap();
	let blocked: IpAddr = "5.6.7.8".parse().unwrap();
	let call_from = async |ip: IpAddr| {
		setup
			.make_api_call_from_ip(
				ApiRequest::<ListUserWorkspacesRequest>::builder()
					.headers(ListUserWorkspacesRequestHeaders {
						authorization: token_bearer.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
				ip,
			)
			.await
			.status_code()
	};

	assert!(
		call_from(blocked).await.is_success(),
		"an unrestricted token should accept any IP"
	);

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateApiTokenRequest>::builder()
				.path(UpdateApiTokenPath {
					token_id: created.id,
				})
				.headers(UpdateApiTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateApiTokenRequest {
					token: UserApiToken {
						name: random_name(8),
						permissions,
						token_nbf: None,
						token_exp: None,
						allowed_ips: Some(vec![IpNetwork::from(allowed)]),
						created: time::OffsetDateTime::now_utc(),
					},
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateApiTokenResponse));

	assert!(
		call_from(blocked).await.is_client_error(),
		"the IP restriction should apply to the next call"
	);
	assert!(
		call_from(allowed).await.is_success(),
		"the listed IP should still be accepted"
	);
}
