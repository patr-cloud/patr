//! API token tests, grouped by what they exercise.

use std::{
	collections::{BTreeMap, BTreeSet},
	str::FromStr,
};

use models::{
	api::{user::*, workspace::rbac::role::*},
	rbac::WorkspacePermission,
	utils::Uuid,
};

use crate::prelude::*;

/// What a token may do, and how the ceiling clamps.
pub mod ceiling;
/// Minting, listing, reading, renaming, revoking, regenerating.
pub mod crud;
/// Cross-user isolation.
pub mod isolation;
/// The nbf/exp window, malformed tokens, and IP restrictions.
pub mod validity;

/// Probe a ModifyRoles-gated action (create a role) using a raw API token.
/// Returns the raw response so callers can assert the authz outcome.
pub(super) async fn probe_modify_roles(
	setup: &TestSetup,
	token: &str,
	workspace_id: Uuid,
	view_perm: Uuid,
) -> axum_test::TestResponse {
	setup
		.make_api_call(
			ApiRequest::<CreateNewRoleRequest>::builder()
				.path(CreateNewRolePath { workspace_id })
				.headers(CreateNewRoleRequestHeaders {
					authorization: BearerToken::from_str(token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateNewRoleRequest {
					role: Role {
						name: random_name(8),
						description: "cascade probe".to_string(),
						is_immutable: false,
					},
					permissions: BTreeSet::from([view_perm]),
				})
				.build(),
		)
		.await
}

/// Call the API-token entrypoint with the given raw token (lists workspaces).
/// Returns the raw response so callers can assert the auth outcome.
pub(super) async fn call_with_token(setup: &TestSetup, token: &str) -> axum_test::TestResponse {
	setup
		.make_api_call(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: BearerToken::from_str(token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
}

/// Mint an API token via the web dashboard, returning the raw response.
pub(super) async fn mint_token_raw(
	setup: &TestSetup,
	token: &BearerToken,
	permissions: BTreeMap<Uuid, WorkspacePermission>,
	token_nbf: Option<time::OffsetDateTime>,
	token_exp: Option<time::OffsetDateTime>,
	allowed_ips: Option<Vec<ipnetwork::IpNetwork>>,
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<CreateApiTokenRequest>::builder()
				.headers(CreateApiTokenRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateApiTokenRequest {
					token: UserApiToken {
						name: random_name(8),
						permissions,
						token_nbf,
						token_exp,
						allowed_ips,
						created: time::OffsetDateTime::now_utc(),
					},
				})
				.build(),
		)
		.await
}
