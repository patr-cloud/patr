use models::{ApiSuccessResponseBody, api::workspace::*, rbac::Permission, utils::Uuid};

use crate::prelude::*;

pub mod container_registry;
pub mod deployment;
pub mod domain;
pub mod managed_url;
pub mod rbac;
pub mod resources_info;
pub mod runner;
pub mod service_account;
pub mod volume;

#[tokio::test]
async fn create_workspace_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let workspace = setup.create_test_workspace(&user.access_token).await;
	assert!(!workspace.name.is_empty());
}

#[tokio::test]
async fn create_workspace_duplicate_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateWorkspaceRequest>::builder()
				.headers(CreateWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateWorkspaceRequest {
					name: workspace.name.clone(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for duplicate workspace name"
	);
}

#[tokio::test]
async fn create_workspace_invalid_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateWorkspaceRequest>::builder()
				.headers(CreateWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateWorkspaceRequest {
					name: "!!!".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for invalid workspace name"
	);
}

#[tokio::test]
async fn create_workspace_name_too_short() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateWorkspaceRequest>::builder()
				.headers(CreateWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateWorkspaceRequest {
					name: "abc".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"workspace name shorter than 4 chars should be rejected"
	);
}

#[tokio::test]
async fn create_workspace_name_too_long() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateWorkspaceRequest>::builder()
				.headers(CreateWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateWorkspaceRequest {
					name: "a".repeat(256),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"workspace name longer than 255 chars should be rejected"
	);
}

#[tokio::test]
async fn create_workspace_name_special_chars() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateWorkspaceRequest>::builder()
				.headers(CreateWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateWorkspaceRequest {
					name: "weird/name@with#chars".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"workspace name with chars outside RESOURCE_NAME_REGEX should be rejected"
	);
}

#[tokio::test]
async fn get_workspace_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetWorkspaceInfoRequest>::builder()
				.path(GetWorkspaceInfoPath {
					workspace_id: workspace.id,
				})
				.headers(GetWorkspaceInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetWorkspaceInfoResponse>>();

	assert_eq!(workspace.name, response.response.workspace.name);
	assert_eq!(workspace.id, response.response.workspace.id);
}

#[tokio::test]
async fn get_workspace_info_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetWorkspaceInfoRequest>::builder()
				.path(GetWorkspaceInfoPath {
					workspace_id: workspace.id,
				})
				.headers(GetWorkspaceInfoRequestHeaders {
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

#[tokio::test]
async fn get_workspace_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetWorkspaceInfoRequest>::builder()
				.path(GetWorkspaceInfoPath {
					workspace_id: Uuid::nil(),
				})
				.headers(GetWorkspaceInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for nonexistent workspace"
	);
}

#[tokio::test]
async fn update_workspace_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let new_name = random_name(8);

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateWorkspaceInfoRequest>::builder()
				.path(UpdateWorkspaceInfoPath {
					workspace_id: workspace.id,
				})
				.headers(UpdateWorkspaceInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateWorkspaceInfoRequest {
					name: new_name.clone(),
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateWorkspaceInfoResponse));

	// Verify
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetWorkspaceInfoRequest>::builder()
				.path(GetWorkspaceInfoPath {
					workspace_id: workspace.id,
				})
				.headers(GetWorkspaceInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetWorkspaceInfoResponse>>();

	assert_eq!(new_name, response.response.workspace.name);
}

#[tokio::test]
async fn update_workspace_name_conflict() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace_a = setup.create_test_workspace(&user.access_token).await;
	let workspace_b = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateWorkspaceInfoRequest>::builder()
				.path(UpdateWorkspaceInfoPath {
					workspace_id: workspace_b.id,
				})
				.headers(UpdateWorkspaceInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateWorkspaceInfoRequest {
					name: workspace_a.name.clone(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"renaming to a taken name should fail"
	);
}

#[tokio::test]
async fn update_workspace_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let owner = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&owner.access_token).await;
	let other_user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateWorkspaceInfoRequest>::builder()
				.path(UpdateWorkspaceInfoPath {
					workspace_id: workspace.id,
				})
				.headers(UpdateWorkspaceInfoRequestHeaders {
					authorization: other_user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateWorkspaceInfoRequest {
					name: random_name(8),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"non-member should not be able to update workspace"
	);
}

#[tokio::test]
async fn update_workspace_denied_without_edit_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;

	// A member with only ViewRoles — no EditWorkspace.
	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::ViewRoles)],
		)
		.await;
	let member = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateWorkspaceInfoRequest>::builder()
				.path(UpdateWorkspaceInfoPath {
					workspace_id: workspace.id,
				})
				.headers(UpdateWorkspaceInfoRequestHeaders {
					authorization: member.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateWorkspaceInfoRequest {
					name: random_name(8),
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"a member without editWorkspace should not be able to rename the workspace, got {}",
		response.status_code()
	);
}

#[tokio::test]
#[ignore = "workspace deletion needs audit_log FK redesign"]
async fn delete_workspace_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteWorkspaceRequest>::builder()
				.path(DeleteWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(DeleteWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteWorkspaceResponse));

	// Verify it's gone
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetWorkspaceInfoRequest>::builder()
				.path(GetWorkspaceInfoPath {
					workspace_id: workspace.id,
				})
				.headers(GetWorkspaceInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"deleted workspace should not be found"
	);
}

#[tokio::test]
async fn delete_workspace_not_super_admin() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let other_user = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteWorkspaceRequest>::builder()
				.path(DeleteWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(DeleteWorkspaceRequestHeaders {
					authorization: other_user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"non-super-admin should not be able to delete workspace"
	);
}

#[tokio::test]
async fn is_name_available_true() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let name = random_name(8);
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<IsWorkspaceNameAvailableRequest>::builder()
				.headers(IsWorkspaceNameAvailableRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.query(IsWorkspaceNameAvailableQuery { name })
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<IsWorkspaceNameAvailableResponse>>();

	assert!(
		response.response.available,
		"unused name should be available"
	);
}

#[tokio::test]
async fn is_name_available_folds_case() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	// `workspace.name` is CITEXT and `workspace_uq_name` indexes it directly,
	// so a case-variant of an existing name collides on insert. The
	// availability check has to agree, or it green-lights a name that
	// creation then rejects on the unique index.
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<IsWorkspaceNameAvailableRequest>::builder()
				.headers(IsWorkspaceNameAvailableRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.query(IsWorkspaceNameAvailableQuery {
					name: workspace.name.to_uppercase(),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<IsWorkspaceNameAvailableResponse>>();

	assert!(
		!response.response.available,
		"a case-variant of a taken name must not be reported as available"
	);
}

#[tokio::test]
async fn is_name_available_false() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<IsWorkspaceNameAvailableRequest>::builder()
				.headers(IsWorkspaceNameAvailableRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.query(IsWorkspaceNameAvailableQuery {
					name: workspace.name,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<IsWorkspaceNameAvailableResponse>>();

	assert!(
		!response.response.available,
		"taken name should not be available"
	);
}

#[tokio::test]
async fn is_name_available_rejects_malformed_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	// `!!!` fails RESOURCE_NAME_REGEX (special chars + under the 4-char min),
	// so the query preprocessor should reject it rather than reporting
	// availability for a name that could never be created.
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<IsWorkspaceNameAvailableRequest>::builder()
				.headers(IsWorkspaceNameAvailableRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.query(IsWorkspaceNameAvailableQuery {
					name: "!!!".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"malformed name should be rejected, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn is_name_available_after_soft_delete() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	// Soft-delete the workspace directly (the DELETE endpoint is blocked on an
	// audit_log FK redesign). The name should then free up, since the partial
	// unique index only covers `deleted IS NULL`.
	setup
		.execute_sql(&format!(
			"UPDATE workspace SET deleted = NOW() WHERE name = '{}';",
			workspace.name
		))
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<IsWorkspaceNameAvailableRequest>::builder()
				.headers(IsWorkspaceNameAvailableRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.query(IsWorkspaceNameAvailableQuery {
					name: workspace.name.clone(),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<IsWorkspaceNameAvailableResponse>>();

	assert!(
		response.response.available,
		"a soft-deleted workspace name should be available again"
	);
}

#[tokio::test]
async fn concurrent_create_same_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;

	let name = random_name(8);

	// Fire 5 concurrent create_workspace calls with the same name. Exactly
	// one should succeed; the rest should be rejected by the unique
	// constraint on workspace.name.
	let req = || {
		let body = CreateWorkspaceRequest { name: name.clone() };
		setup.make_web_dashboard_call(
			ApiRequest::<CreateWorkspaceRequest>::builder()
				.headers(CreateWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(body)
				.build(),
		)
	};
	let responses = futures::future::join_all([req(), req(), req(), req(), req()]).await;
	let statuses: Vec<_> = responses.iter().map(|r| r.status_code()).collect();
	let successes = statuses.iter().filter(|s| s.is_success()).count();
	let failures = statuses.iter().filter(|s| s.is_client_error()).count();

	assert_eq!(
		(successes, failures),
		(1, 4),
		"expected exactly 1 success and 4 client errors from 5 concurrent same-name create_workspace; got {statuses:?}"
	);
}

/// Call `LeaveWorkspace` as the given user.
async fn leave_workspace(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<LeaveWorkspaceRequest>::builder()
				.path(LeaveWorkspacePath { workspace_id })
				.headers(LeaveWorkspaceRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
}

#[tokio::test]
async fn member_can_leave_workspace() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let role = setup
		.create_test_role(&admin.access_token, workspace.id)
		.await;
	let member = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = leave_workspace(&setup, &member.access_token, workspace.id).await;
	assert_eq!(response.status_code(), StatusCode::ACCEPTED);

	// Having left, the member can no longer access the workspace.
	let after = setup
		.make_web_dashboard_call(
			ApiRequest::<GetWorkspaceInfoRequest>::builder()
				.path(GetWorkspaceInfoPath {
					workspace_id: workspace.id,
				})
				.headers(GetWorkspaceInfoRequestHeaders {
					authorization: member.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		after.status_code().is_client_error(),
		"a user who left should lose access to the workspace"
	);
}

#[tokio::test]
async fn owner_cannot_leave_workspace() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;

	let response = leave_workspace(&setup, &admin.access_token, workspace.id).await;
	assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn non_member_cannot_leave_workspace() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let stranger = setup.create_test_user().await;

	let response = leave_workspace(&setup, &stranger.access_token, workspace.id).await;
	assert!(
		response.status_code().is_client_error(),
		"a non-member should not be able to leave a workspace"
	);
}
