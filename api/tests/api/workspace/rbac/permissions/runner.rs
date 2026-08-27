use std::collections::BTreeMap;

use models::{
	ApiSuccessResponseBody,
	api::workspace::runner::*,
	rbac::{Permission, RunnerPermission},
};

use super::{all, exclude, grant, include, resources_scope, setup_permission_test};
use crate::prelude::*;

#[tokio::test]
async fn runner_create_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Runner(RunnerPermission::Create), all())],
	)
	.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<AddRunnerToWorkspaceRequest>::builder()
				.path(AddRunnerToWorkspacePath {
					workspace_id: ws_id,
				})
				.headers(AddRunnerToWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(AddRunnerToWorkspaceRequest {
					name: random_name(8),
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn runner_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(setup.get_permission_id(Permission::ViewRoles), all());
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetRunnerInfoRequest>::builder()
				.path(GetRunnerInfoPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(GetRunnerInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without runner::view should be denied"
	);
}

#[tokio::test]
async fn runner_include_grants_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner1 = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let runner2 = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Runner(RunnerPermission::View)),
		include(&[runner1.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grant(
			&admin.access_token,
			workspace.id,
			grant(role.id, resources_scope(&[runner1.id])),
		)
		.await;

	let r1 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetRunnerInfoRequest>::builder()
				.path(GetRunnerInfoPath {
					workspace_id: workspace.id,
					runner_id: runner1.id,
				})
				.headers(GetRunnerInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r1.status_code().is_success());

	let runner2 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetRunnerInfoRequest>::builder()
				.path(GetRunnerInfoPath {
					workspace_id: workspace.id,
					runner_id: runner2.id,
				})
				.headers(GetRunnerInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(runner2.status_code().is_client_error());
}

#[tokio::test]
async fn runner_grant_omitting_a_resource_denies_it() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner1 = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let runner2 = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Runner(RunnerPermission::View)),
		include(&[runner1.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grant(
			&admin.access_token,
			workspace.id,
			grant(role.id, resources_scope(&[runner1.id])),
		)
		.await;

	let r1 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetRunnerInfoRequest>::builder()
				.path(GetRunnerInfoPath {
					workspace_id: workspace.id,
					runner_id: runner1.id,
				})
				.headers(GetRunnerInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r1.status_code().is_success(),
		"runner1 should be accessible"
	);

	let runner2 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetRunnerInfoRequest>::builder()
				.path(GetRunnerInfoPath {
					workspace_id: workspace.id,
					runner_id: runner2.id,
				})
				.headers(GetRunnerInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		runner2.status_code().is_client_error(),
		"runner2 should be excluded"
	);
}

/// Create does not imply View: a create-only member can add a runner but cannot
/// read it back.
#[tokio::test]
async fn runner_create_does_not_grant_view() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Runner(RunnerPermission::Create), all())],
	)
	.await;

	let created = setup
		.make_web_dashboard_call(
			ApiRequest::<AddRunnerToWorkspaceRequest>::builder()
				.path(AddRunnerToWorkspacePath {
					workspace_id: ws_id,
				})
				.headers(AddRunnerToWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(AddRunnerToWorkspaceRequest {
					name: random_name(8),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<AddRunnerToWorkspaceResponse>>();

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetRunnerInfoRequest>::builder()
				.path(GetRunnerInfoPath {
					workspace_id: ws_id,
					runner_id: created.response.id.id,
				})
				.headers(GetRunnerInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"create-only member should not be able to view the runner"
	);
}

/// A member with no runner permission gets a membership-gated list that
/// succeeds but is View-filtered to empty — not a 403.
#[tokio::test]
async fn runner_no_permission_list_returns_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, ws_id, user_b) =
		setup_permission_test(&setup, vec![(Permission::ViewRoles, all())]).await;
	let _runner = setup.create_test_runner(&admin.access_token, ws_id).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
				.path(ListRunnersForWorkspacePath {
					workspace_id: ws_id,
				})
				.headers(ListRunnersForWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListRunnersForWorkspaceResponse>>();
	assert!(
		response.response.runners.is_empty(),
		"a member without runner View should see an empty list, not a 403"
	);
}

/// The ingress-token endpoint requires Execute: a View-only member is denied.
#[tokio::test]
async fn runner_ingress_token_requires_execute() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Runner(RunnerPermission::View), all())],
	)
	.await;
	let runner = setup.create_test_runner(&admin.access_token, ws_id).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetIngressTokenForRunnerRequest>::builder()
				.path(GetIngressTokenForRunnerPath {
					workspace_id: ws_id,
					runner_id: runner.id,
				})
				.headers(GetIngressTokenForRunnerRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"a View-only member should be denied the ingress token (requires Execute)"
	);
}

/// A non-member cannot reach another workspace's runners at all.
#[tokio::test]
async fn runner_non_member_denied() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let _runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let outsider = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
				.path(ListRunnersForWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListRunnersForWorkspaceRequestHeaders {
					authorization: outsider.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"a non-member should be denied access to the workspace's runners"
	);
}

#[tokio::test]
async fn runner_view_does_not_grant_delete() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Runner(RunnerPermission::View)),
		include(&[runner.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grant(
			&admin.access_token,
			workspace.id,
			grant(role.id, resources_scope(&[runner.id])),
		)
		.await;

	let r_view = setup
		.make_web_dashboard_call(
			ApiRequest::<GetRunnerInfoRequest>::builder()
				.path(GetRunnerInfoPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(GetRunnerInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r_view.status_code().is_success());

	let r_delete = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteRunnerRequest>::builder()
				.path(DeleteRunnerPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(DeleteRunnerRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r_delete.status_code().is_client_error(),
		"view permission should not grant delete"
	);
}

#[tokio::test]
async fn runner_view_does_not_grant_create() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Runner(RunnerPermission::View)),
		all(),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	// View should succeed.
	let r_view = setup
		.make_web_dashboard_call(
			ApiRequest::<GetRunnerInfoRequest>::builder()
				.path(GetRunnerInfoPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(GetRunnerInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r_view.status_code().is_success());

	// Create should fail.
	let r_create = setup
		.make_web_dashboard_call(
			ApiRequest::<AddRunnerToWorkspaceRequest>::builder()
				.path(AddRunnerToWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(AddRunnerToWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(AddRunnerToWorkspaceRequest {
					name: random_name(8),
				})
				.build(),
		)
		.await;
	assert!(
		r_create.status_code().is_client_error(),
		"view permission should not grant create"
	);
}
