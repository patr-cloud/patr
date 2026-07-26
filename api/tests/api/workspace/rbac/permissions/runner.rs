use std::collections::BTreeMap;

use models::{
	api::workspace::runner::*,
	rbac::{Permission, RunnerPermission, WorkspacePermission},
};

use super::{all, exclude, include, setup_permission_test};
use crate::prelude::*;

#[tokio::test]
async fn runner_create_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Runner(RunnerPermission::Create), all())],
	)
	.await;

	// The CLI half of the flow (create the consent link) runs off an API token.
	let api_token = setup
		.create_test_api_token(
			&admin.access_token,
			BTreeMap::from([(ws_id, WorkspacePermission::SuperAdmin)]),
		)
		.await;
	let link = setup
		.make_api_call(
			ApiRequest::<CreateRunnerLinkRequest>::builder()
				.path(CreateRunnerLinkPath {
					workspace_id: ws_id,
				})
				.headers(CreateRunnerLinkRequestHeaders {
					authorization: BearerToken::from_str(&api_token.token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateRunnerLinkRequest {
					version: "0.1.0".parse().unwrap(),
					os: "linux".to_string(),
					arch: "x86_64".to_string(),
					hostname: random_name(8),
					private_ip: "127.0.0.1".parse().unwrap(),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateRunnerLinkResponse>>()
		.response;

	// user_b holds only runner::create — enough to approve the link.
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ApproveRunnerLinkRequest>::builder()
				.path(ApproveRunnerLinkPath {
					workspace_id: ws_id,
					user_code: link.user_code,
				})
				.headers(ApproveRunnerLinkRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ApproveRunnerLinkRequest {
					runner_name: random_name(8),
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
		.make_api_call(
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
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let r1 = setup
		.make_api_call(
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
		.make_api_call(
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
async fn runner_exclude_denies_only_listed_resource() {
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
		exclude(&[runner2.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let r1 = setup
		.make_api_call(
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
		.make_api_call(
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
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let r_view = setup
		.make_api_call(
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
		.make_api_call(
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
