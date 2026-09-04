use std::collections::BTreeMap;

use apalis::prelude::Data;
use apalis_cron::Tick;
use models::{
	ApiErrorResponseBody,
	api::{
		user::{
			AcceptWorkspaceInviteRequest,
			AcceptWorkspaceInviteRequestHeaders,
			AcceptWorkspaceInviteResponse,
			PreviewWorkspaceInviteRequest,
			PreviewWorkspaceInviteRequestHeaders,
			PreviewWorkspaceInviteResponse,
		},
		workspace::rbac::user::*,
	},
	rbac::Permission,
};

use crate::prelude::*;

/// Pull the plaintext invite token out of an accept link. The token is stored
/// hashed, so the link handed back on invite/resend is the only place it can be
/// read — the same affordance the dashboard's "copy link" button uses.
fn token_from_accept_url(accept_url: &str) -> String {
	accept_url
		.split_once("token=")
		.expect("accept_url should carry the invite token")
		.1
		.to_string()
}

/// Invite an email to a workspace with a single role, returning the raw
/// response.
async fn invite(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	email: &str,
	roles: Vec<Uuid>,
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<InviteUserToWorkspaceRequest>::builder()
				.path(InviteUserToWorkspacePath { workspace_id })
				.headers(InviteUserToWorkspaceRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(InviteUserToWorkspaceRequest {
					email: email.to_string(),
					roles: roles
						.into_iter()
						.map(|role_id| RoleGrant {
							role_id,
							resource_id: workspace_id,
						})
						.collect(),
				})
				.build(),
		)
		.await
}

/// Accept an invite as the given user.
async fn accept(
	setup: &TestSetup,
	token: &BearerToken,
	invite_id: Uuid,
	invite_token: &str,
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<AcceptWorkspaceInviteRequest>::builder()
				.headers(AcceptWorkspaceInviteRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(AcceptWorkspaceInviteRequest {
					invite_id,
					token: invite_token.to_string(),
				})
				.build(),
		)
		.await
}

/// Preview an invite as the given user, without consuming it.
async fn preview(
	setup: &TestSetup,
	token: &BearerToken,
	invite_id: Uuid,
	invite_token: &str,
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<PreviewWorkspaceInviteRequest>::builder()
				.headers(PreviewWorkspaceInviteRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(PreviewWorkspaceInviteRequest {
					invite_id,
					token: invite_token.to_string(),
				})
				.build(),
		)
		.await
}

/// The pending invites for a workspace.
async fn list_invites(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
) -> Vec<WithId<WorkspaceInvite>> {
	setup
		.make_web_dashboard_call(
			ApiRequest::<ListWorkspaceInvitesRequest>::builder()
				.path(ListWorkspaceInvitesPath { workspace_id })
				.headers(ListWorkspaceInvitesRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListWorkspaceInvitesResponse>>()
		.response
		.invites
}

/// The set of (user_id -> role_ids) currently in a workspace.
async fn members(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
) -> BTreeMap<Uuid, Vec<RoleGrant>> {
	setup
		.make_web_dashboard_call(
			ApiRequest::<ListUsersInWorkspaceRequest>::builder()
				.path(ListUsersInWorkspacePath { workspace_id })
				.headers(ListUsersInWorkspaceRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListUsersInWorkspaceResponse>>()
		.response
		.users
		.into_iter()
		.map(|member| (member.user.id, member.roles))
		.collect()
}

#[tokio::test]
async fn invite_and_accept_adds_member_with_role() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let invitee = setup.create_test_user().await;

	let invite_response = invite(
		&setup,
		&admin.access_token,
		workspace.id,
		&invitee.email.clone(),
		vec![role.id],
	)
	.await;
	assert_eq!(invite_response.status_code(), StatusCode::CREATED);
	let created = invite_response
		.json::<ApiSuccessResponseBody<InviteUserToWorkspaceResponse>>()
		.response;
	let invite_id = created.id.id;
	let invite_token = token_from_accept_url(&created.accept_url);

	let accept_response = accept(&setup, &invitee.access_token, invite_id, &invite_token).await;
	assert_eq!(accept_response.status_code(), StatusCode::ACCEPTED);
	assert_eq!(
		accept_response
			.json::<ApiSuccessResponseBody<AcceptWorkspaceInviteResponse>>()
			.response
			.id
			.id,
		workspace.id
	);

	let members = members(&setup, &admin.access_token, workspace.id).await;
	assert_eq!(
		members
			.get(&invitee.user_id)
			.map(|grants| grants.iter().map(|grant| grant.role_id).collect::<Vec<_>>()),
		Some(vec![role.id]),
		"invitee should be a member with the invited role"
	);
}

#[tokio::test]
async fn invite_requires_modify_roles() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;

	// A role that can view but not modify roles.
	let view_role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::ViewRoles)],
		)
		.await;
	let member = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, view_role.id)
		.await;

	let outsider = setup.create_test_user().await;
	let response = invite(
		&setup,
		&member.access_token,
		workspace.id,
		&outsider.email.clone(),
		vec![view_role.id],
	)
	.await;

	assert!(
		response.status_code().is_client_error(),
		"a member without ModifyRoles should not be able to invite"
	);
}

#[tokio::test]
async fn invite_existing_member_fails() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let member = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = invite(
		&setup,
		&admin.access_token,
		workspace.id,
		&member.email.clone(),
		vec![role.id],
	)
	.await;

	assert_eq!(response.status_code(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn invite_invalid_role_fails() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let invitee = setup.create_test_user().await;

	// A role id that does not belong to this workspace.
	let response = invite(
		&setup,
		&admin.access_token,
		workspace.id,
		&invitee.email.clone(),
		vec![Uuid::new_v4()],
	)
	.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn list_and_revoke_invite() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let invitee = setup.create_test_user().await;

	let (invite_id, invite_token) = invite_returning_id_and_token(
		&setup,
		&admin.access_token,
		workspace.id,
		&invitee.email.clone(),
		vec![role.id],
	)
	.await;

	let invites = list_invites(&setup, &admin.access_token, workspace.id).await;
	assert_eq!(invites.len(), 1);
	assert_eq!(invites[0].id, invite_id);
	assert_eq!(invites[0].data.email, invitee.email.clone());

	let revoke_response = setup
		.make_web_dashboard_call(
			ApiRequest::<RevokeWorkspaceInviteRequest>::builder()
				.path(RevokeWorkspaceInvitePath {
					workspace_id: workspace.id,
					invite_id,
				})
				.headers(RevokeWorkspaceInviteRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(revoke_response.status_code(), StatusCode::OK);

	assert!(
		list_invites(&setup, &admin.access_token, workspace.id)
			.await
			.is_empty(),
		"invite should be gone after revoke"
	);

	// A revoked invite can no longer be accepted.
	let accept_response = accept(&setup, &invitee.access_token, invite_id, &invite_token).await;
	assert_eq!(accept_response.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn accept_wrong_account_fails() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let invitee = setup.create_test_user().await;
	let other = setup.create_test_user().await;

	let (invite_id, invite_token) = invite_returning_id_and_token(
		&setup,
		&admin.access_token,
		workspace.id,
		&invitee.email.clone(),
		vec![role.id],
	)
	.await;

	// A different logged-in user (not the invited email) cannot accept.
	let response = accept(&setup, &other.access_token, invite_id, &invite_token).await;
	assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn accept_expired_fails() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let invitee = setup.create_test_user().await;

	let (invite_id, invite_token) = invite_returning_id_and_token(
		&setup,
		&admin.access_token,
		workspace.id,
		&invitee.email.clone(),
		vec![role.id],
	)
	.await;

	// Backdate the expiry so the invite is stale.
	setup
		.execute_sql(&format!(
			"UPDATE workspace_user_invite SET token_expiry = NOW() - INTERVAL '1 day' WHERE id = '{invite_id}'"
		))
		.await;

	let response = accept(&setup, &invitee.access_token, invite_id, &invite_token).await;
	assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn accept_bad_token_fails() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let invitee = setup.create_test_user().await;

	let (invite_id, invite_token) = invite_returning_id_and_token(
		&setup,
		&admin.access_token,
		workspace.id,
		&invitee.email.clone(),
		vec![role.id],
	)
	.await;

	let response = accept(
		&setup,
		&invitee.access_token,
		invite_id,
		"not-the-real-token",
	)
	.await;
	assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

	// The real token still works after a failed attempt.
	let ok = accept(&setup, &invitee.access_token, invite_id, &invite_token).await;
	assert_eq!(ok.status_code(), StatusCode::ACCEPTED);
}

/// Invite an email and return the created invite's id.
async fn invite_returning_id(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	email: &str,
	roles: Vec<Uuid>,
) -> Uuid {
	invite_returning_id_and_token(setup, token, workspace_id, email, roles)
		.await
		.0
}

/// Invite an email and return the created invite's id along with its plaintext
/// token.
async fn invite_returning_id_and_token(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	email: &str,
	roles: Vec<Uuid>,
) -> (Uuid, String) {
	let response = invite(setup, token, workspace_id, email, roles)
		.await
		.json::<ApiSuccessResponseBody<InviteUserToWorkspaceResponse>>()
		.response;

	(response.id.id, token_from_accept_url(&response.accept_url))
}

#[tokio::test]
async fn invite_duplicate_email_fails() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let invitee = setup.create_test_user().await;
	let email = invitee.email.clone();

	let first = invite(
		&setup,
		&admin.access_token,
		workspace.id,
		&email,
		vec![role.id],
	)
	.await;
	assert_eq!(first.status_code(), StatusCode::CREATED);

	// A second invite to the same pending email is rejected.
	let second = invite(
		&setup,
		&admin.access_token,
		workspace.id,
		&email,
		vec![role.id],
	)
	.await;
	assert_eq!(second.status_code(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn update_invite_roles_works() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role_a = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let role_b = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let invitee = setup.create_test_user().await;

	let (invite_id, invite_token) = invite_returning_id_and_token(
		&setup,
		&admin.access_token,
		workspace.id,
		&invitee.email.clone(),
		vec![role_a.id],
	)
	.await;

	// Swap the invited role from A to B without resending.
	let update = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateWorkspaceInviteRolesRequest>::builder()
				.path(UpdateWorkspaceInviteRolesPath {
					workspace_id: workspace.id,
					invite_id,
				})
				.headers(UpdateWorkspaceInviteRolesRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateWorkspaceInviteRolesRequest {
					roles: vec![RoleGrant {
						role_id: role_b.id,
						resource_id: workspace.id,
					}],
				})
				.build(),
		)
		.await;
	assert_eq!(update.status_code(), StatusCode::OK);

	let invites = list_invites(&setup, &admin.access_token, workspace.id).await;
	assert_eq!(
		invites[0]
			.data
			.roles
			.iter()
			.map(|grant| grant.role_id)
			.collect::<Vec<_>>(),
		vec![role_b.id]
	);

	// The original link still works and grants the updated role.
	let accept_response = accept(&setup, &invitee.access_token, invite_id, &invite_token).await;
	assert_eq!(accept_response.status_code(), StatusCode::ACCEPTED);

	let members = members(&setup, &admin.access_token, workspace.id).await;
	assert_eq!(
		members
			.get(&invitee.user_id)
			.map(|grants| grants.iter().map(|grant| grant.role_id).collect::<Vec<_>>()),
		Some(vec![role_b.id])
	);
}

#[tokio::test]
async fn resend_invite_works() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let invitee = setup.create_test_user().await;

	let (invite_id, invite_token) = invite_returning_id_and_token(
		&setup,
		&admin.access_token,
		workspace.id,
		&invitee.email.clone(),
		vec![role.id],
	)
	.await;

	let resend = setup
		.make_web_dashboard_call(
			ApiRequest::<ResendWorkspaceInviteRequest>::builder()
				.path(ResendWorkspaceInvitePath {
					workspace_id: workspace.id,
					invite_id,
				})
				.headers(ResendWorkspaceInviteRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(resend.status_code(), StatusCode::OK);
	let resent_token = token_from_accept_url(
		&resend
			.json::<ApiSuccessResponseBody<ResendWorkspaceInviteResponse>>()
			.response
			.accept_url,
	);

	// Resending mints a fresh token, so the one from the original invite is dead.
	let stale = accept(&setup, &invitee.access_token, invite_id, &invite_token).await;
	assert_eq!(stale.status_code(), StatusCode::BAD_REQUEST);

	// The regenerated token accepts and grants the roles.
	let accept_response = accept(&setup, &invitee.access_token, invite_id, &resent_token).await;
	assert_eq!(accept_response.status_code(), StatusCode::ACCEPTED);
	assert_eq!(
		members(&setup, &admin.access_token, workspace.id)
			.await
			.get(&invitee.user_id)
			.map(|grants| grants.iter().map(|grant| grant.role_id).collect::<Vec<_>>()),
		Some(vec![role.id])
	);
}

#[tokio::test]
async fn preview_returns_workspace_name() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let invitee = setup.create_test_user().await;

	let (invite_id, invite_token) = invite_returning_id_and_token(
		&setup,
		&admin.access_token,
		workspace.id,
		&invitee.email.clone(),
		vec![role.id],
	)
	.await;

	let response = preview(&setup, &invitee.access_token, invite_id, &invite_token).await;
	assert_eq!(response.status_code(), StatusCode::OK);
	assert_eq!(
		response
			.json::<ApiSuccessResponseBody<PreviewWorkspaceInviteResponse>>()
			.response
			.workspace_name,
		workspace.name,
		"preview should name the workspace being joined"
	);

	// Preview is read-only: the invite is still pending and still acceptable.
	assert_eq!(
		list_invites(&setup, &admin.access_token, workspace.id)
			.await
			.len(),
		1,
		"preview must not consume the invite"
	);
	let accept_response = accept(&setup, &invitee.access_token, invite_id, &invite_token).await;
	assert_eq!(accept_response.status_code(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn preview_bad_token_fails() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let invitee = setup.create_test_user().await;

	let (invite_id, invite_token) = invite_returning_id_and_token(
		&setup,
		&admin.access_token,
		workspace.id,
		&invitee.email.clone(),
		vec![role.id],
	)
	.await;

	let response = preview(
		&setup,
		&invitee.access_token,
		invite_id,
		"not-the-real-token",
	)
	.await;
	assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

	// An unknown invite id looks the same as a bad token.
	let unknown = preview(&setup, &invitee.access_token, Uuid::new_v4(), &invite_token).await;
	assert_eq!(unknown.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn preview_expired_fails() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let invitee = setup.create_test_user().await;

	let (invite_id, invite_token) = invite_returning_id_and_token(
		&setup,
		&admin.access_token,
		workspace.id,
		&invitee.email.clone(),
		vec![role.id],
	)
	.await;

	setup
		.execute_sql(&format!(
			"UPDATE workspace_user_invite SET token_expiry = NOW() - INTERVAL '1 day' WHERE id = '{invite_id}'"
		))
		.await;

	let response = preview(&setup, &invitee.access_token, invite_id, &invite_token).await;
	assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn repeated_wrong_tokens_do_not_lock_invite() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let invitee = setup.create_test_user().await;

	let (invite_id, invite_token) = invite_returning_id_and_token(
		&setup,
		&admin.access_token,
		workspace.id,
		&invitee.email.clone(),
		vec![role.id],
	)
	.await;

	// A ceiling on a 256-bit token would only let a third party who knows the
	// `invite_id` lock the invitee out, so there deliberately isn't one.
	for _ in 0..8 {
		let response = accept(&setup, &invitee.access_token, invite_id, "wrong-token").await;
		assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

		let response = preview(&setup, &invitee.access_token, invite_id, "wrong-token").await;
		assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
	}

	let accept_response = accept(&setup, &invitee.access_token, invite_id, &invite_token).await;
	assert_eq!(
		accept_response.status_code(),
		StatusCode::ACCEPTED,
		"wrong-token attempts must not lock a valid invite"
	);
}

#[tokio::test]
async fn cleanup_removes_only_long_expired_invites() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;

	let live = invite_returning_id(
		&setup,
		&admin.access_token,
		workspace.id,
		"live@example.com",
		vec![role.id],
	)
	.await;
	let recently_expired = invite_returning_id(
		&setup,
		&admin.access_token,
		workspace.id,
		"recent@example.com",
		vec![role.id],
	)
	.await;
	let long_expired = invite_returning_id(
		&setup,
		&admin.access_token,
		workspace.id,
		"stale@example.com",
		vec![role.id],
	)
	.await;

	// One expired yesterday (still inside the retention window, so the admin
	// can resend it) and one expired well beyond it.
	setup
		.execute_sql(&format!(
			"UPDATE workspace_user_invite SET token_expiry = NOW() - INTERVAL '1 day' \
			 WHERE id = '{recently_expired}'"
		))
		.await;
	setup
		.execute_sql(&format!(
			"UPDATE workspace_user_invite SET token_expiry = NOW() - INTERVAL '30 days' \
			 WHERE id = '{long_expired}'"
		))
		.await;

	api::worker::cleanup_expired_invites::cleanup_expired_invites(
		Tick::default(),
		Data::new(setup.state().clone()),
	)
	.await
	.expect("cleanup failed");

	let remaining = list_invites(&setup, &admin.access_token, workspace.id)
		.await
		.into_iter()
		.map(|invite| invite.id)
		.collect::<Vec<_>>();

	assert!(remaining.contains(&live), "a live invite must survive");
	assert!(
		remaining.contains(&recently_expired),
		"an invite inside the retention window must survive so it can be resent"
	);
	assert!(
		!remaining.contains(&long_expired),
		"a long-expired invite must be cleaned up"
	);

	// The role rows have no ON DELETE CASCADE, so the cron has to clear them
	// itself or they leak.
	let orphaned_roles = sqlx::query_scalar::<_, i64>(&format!(
		"SELECT COUNT(*) FROM workspace_user_invite_role WHERE invite_id = '{long_expired}'"
	))
	.fetch_one(&setup.state().database)
	.await
	.expect("failed to count invite role rows");
	assert_eq!(orphaned_roles, 0, "invite role rows must be cleaned up too");
}

#[tokio::test]
async fn expired_invite_without_token_looks_missing() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let invitee = setup.create_test_user().await;

	let (invite_id, invite_token) = invite_returning_id_and_token(
		&setup,
		&admin.access_token,
		workspace.id,
		&invitee.email.clone(),
		vec![role.id],
	)
	.await;

	setup
		.execute_sql(&format!(
			"UPDATE workspace_user_invite SET token_expiry = NOW() - INTERVAL '1 day' \
			 WHERE id = '{invite_id}'"
		))
		.await;

	// Both errors are a 400, so only the error type tells them apart. Someone
	// holding an invite id but no token must not learn that the invite is real.
	for response in [
		accept(&setup, &invitee.access_token, invite_id, "wrong-token").await,
		preview(&setup, &invitee.access_token, invite_id, "wrong-token").await,
	] {
		assert_eq!(
			response.json::<ApiErrorResponseBody>().error,
			ErrorType::InviteNotFound,
			"an expired invite must not announce itself to a caller without the token"
		);
	}

	// With the real token the caller has earned the more specific error.
	for response in [
		accept(&setup, &invitee.access_token, invite_id, &invite_token).await,
		preview(&setup, &invitee.access_token, invite_id, &invite_token).await,
	] {
		assert_eq!(
			response.json::<ApiErrorResponseBody>().error,
			ErrorType::InviteExpired
		);
	}
}
