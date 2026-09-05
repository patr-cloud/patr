//! One user's token must be invisible to another: cross-user reads and writes
//! answer 404 rather than 403, so they leak nothing.

use std::collections::BTreeMap;

use models::{api::user::*, rbac::WorkspacePermission};

use super::call_with_token;
use crate::prelude::*;

/// One user cannot delete another user's token (404), and the victim's token
/// keeps working.
#[tokio::test]
async fn api_token_cross_user_delete_404() {
	let setup = setup().await.expect("failed to setup test server");
	let user_a = setup.create_test_user().await;
	let ws_a = setup.create_test_workspace(&user_a.access_token).await;
	let user_b = setup.create_test_user().await;
	let token_a = setup
		.create_test_api_token(
			&user_a.access_token,
			BTreeMap::from([(ws_a.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	let resp = setup
		.make_web_dashboard_call(
			ApiRequest::<RevokeApiTokenRequest>::builder()
				.path(RevokeApiTokenPath {
					token_id: token_a.id,
				})
				.headers(RevokeApiTokenRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(
		404,
		resp.status_code().as_u16(),
		"deleting another user's token should be 404"
	);

	assert!(
		call_with_token(&setup, &token_a.token)
			.await
			.status_code()
			.is_success(),
		"the victim's token should still work"
	);
}

/// One user cannot regenerate another user's token (404).
#[tokio::test]
async fn api_token_cross_user_regenerate_404() {
	let setup = setup().await.expect("failed to setup test server");
	let user_a = setup.create_test_user().await;
	let ws_a = setup.create_test_workspace(&user_a.access_token).await;
	let user_b = setup.create_test_user().await;
	let token_a = setup
		.create_test_api_token(
			&user_a.access_token,
			BTreeMap::from([(ws_a.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	let resp = setup
		.make_web_dashboard_call(
			ApiRequest::<RegenerateApiTokenRequest>::builder()
				.path(RegenerateApiTokenPath {
					token_id: token_a.id,
				})
				.headers(RegenerateApiTokenRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(
		404,
		resp.status_code().as_u16(),
		"regenerating another user's token should be 404"
	);
}

/// A PATCH targeting another user's token (IDOR) is 404 and does not wipe the
/// victim's permissions.
#[tokio::test]
async fn api_token_cross_user_patch_idor_404() {
	let setup = setup().await.expect("failed to setup test server");
	let user_a = setup.create_test_user().await;
	let ws_a = setup.create_test_workspace(&user_a.access_token).await;
	let user_b = setup.create_test_user().await;
	let token_a = setup
		.create_test_api_token(
			&user_a.access_token,
			BTreeMap::from([(ws_a.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;

	let resp = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateApiTokenRequest>::builder()
				.path(UpdateApiTokenPath {
					token_id: token_a.id,
				})
				.headers(UpdateApiTokenRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateApiTokenRequest {
					token: UserApiToken {
						name: "idortoken".to_string(),
						permissions: BTreeMap::from([(ws_a.id, WorkspacePermission::SuperAdmin)]),
						token_nbf: None,
						token_exp: None,
						allowed_ips: None,
						created: time::OffsetDateTime::now_utc(),
					},
				})
				.build(),
		)
		.await;
	assert_eq!(
		404,
		resp.status_code().as_u16(),
		"PATCHing another user's token (IDOR) should be 404"
	);

	assert!(
		call_with_token(&setup, &token_a.token)
			.await
			.status_code()
			.is_success(),
		"the victim's token should still work after the IDOR attempt"
	);
}
