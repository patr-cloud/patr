use models::{
	api::workspace::{
		GetWorkspaceInfoPath,
		GetWorkspaceInfoRequest,
		GetWorkspaceInfoRequestHeaders,
		runner::*,
	},
	rbac::{Permission, RunnerPermission},
};

use super::{all, grants, setup_permission_test};
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
			ApiRequest::<CreateRunnerRequest>::builder()
				.path(CreateRunnerPath {
					workspace_id: ws_id,
				})
				.headers(CreateRunnerRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateRunnerRequest {
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

	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::ViewRoles)],
		)
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

	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::Runner(RunnerPermission::View))],
		)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grants(
			&admin.access_token,
			workspace.id,
			grants(role.id, &[runner1.id]),
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

	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::Runner(RunnerPermission::View))],
		)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grants(
			&admin.access_token,
			workspace.id,
			grants(role.id, &[runner1.id]),
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

/// The total count only covers the runners the member can view.
#[tokio::test]
async fn runner_view_include_list_counts_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner1 = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;

	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::Runner(RunnerPermission::View))],
		)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grants(
			&admin.access_token,
			workspace.id,
			grants(role.id, &[runner1.id]),
		)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
				.path(ListRunnersForWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListRunnersForWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert_eq!("1", response.header("x-total-count"));
	let body = response.json::<ApiSuccessResponseBody<ListRunnersForWorkspaceResponse>>();
	assert_eq!(1, body.response.runners.len());
	assert_eq!(runner1.id, body.response.runners[0].id);
}

#[tokio::test]
async fn runner_view_does_not_grant_delete() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;

	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::Runner(RunnerPermission::View))],
		)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grants(
			&admin.access_token,
			workspace.id,
			grants(role.id, &[runner.id]),
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

	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::Runner(RunnerPermission::View))],
		)
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

	let r_create = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateRunnerRequest>::builder()
				.path(CreateRunnerPath {
					workspace_id: workspace.id,
				})
				.headers(CreateRunnerRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateRunnerRequest {
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

/// `runner::view` doesn't let a member regenerate a runner's token, and the
/// runner's token keeps working.
#[tokio::test]
async fn runner_view_does_not_grant_regenerate_token() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Runner(RunnerPermission::View), all())],
	)
	.await;
	let runner = setup.create_test_runner(&admin.access_token, ws_id).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<RegenerateRunnerTokenRequest>::builder()
				.path(RegenerateRunnerTokenPath {
					workspace_id: ws_id,
					runner_id: runner.id,
				})
				.headers(RegenerateRunnerTokenRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(
		StatusCode::UNAUTHORIZED,
		response.status_code(),
		"view permission should not grant regenerating the token"
	);

	setup
		.make_api_call(
			ApiRequest::<GetWorkspaceInfoRequest>::builder()
				.path(GetWorkspaceInfoPath {
					workspace_id: ws_id,
				})
				.headers(GetWorkspaceInfoRequestHeaders {
					authorization: BearerToken::from_str(&runner.token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_status_ok();
}

/// `runner::create` doesn't let a member regenerate an existing runner's
/// token, and the runner's token keeps working.
#[tokio::test]
async fn runner_create_does_not_grant_regenerate_token() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Runner(RunnerPermission::Create), all())],
	)
	.await;
	let runner = setup.create_test_runner(&admin.access_token, ws_id).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<RegenerateRunnerTokenRequest>::builder()
				.path(RegenerateRunnerTokenPath {
					workspace_id: ws_id,
					runner_id: runner.id,
				})
				.headers(RegenerateRunnerTokenRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(
		StatusCode::UNAUTHORIZED,
		response.status_code(),
		"create permission should not grant regenerating the token"
	);

	setup
		.make_api_call(
			ApiRequest::<GetWorkspaceInfoRequest>::builder()
				.path(GetWorkspaceInfoPath {
					workspace_id: ws_id,
				})
				.headers(GetWorkspaceInfoRequestHeaders {
					authorization: BearerToken::from_str(&runner.token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_status_ok();
}

/// `runner::regenerateToken` on one runner regenerates that runner's token and
/// not another's.
#[tokio::test]
async fn runner_regenerate_token_grant_omitting_a_runner_denies_it() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner1 = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let runner2 = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;

	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::Runner(RunnerPermission::RegenerateToken))],
		)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grants(
			&admin.access_token,
			workspace.id,
			grants(role.id, &[runner1.id]),
		)
		.await;
	let regenerate = |runner_id| {
		setup.make_web_dashboard_call(
			ApiRequest::<RegenerateRunnerTokenRequest>::builder()
				.path(RegenerateRunnerTokenPath {
					workspace_id: workspace.id,
					runner_id,
				})
				.headers(RegenerateRunnerTokenRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
	};

	assert_eq!(
		StatusCode::UNAUTHORIZED,
		regenerate(runner2.id).await.status_code(),
		"runner2 should be excluded"
	);
	setup
		.make_api_call(
			ApiRequest::<GetWorkspaceInfoRequest>::builder()
				.path(GetWorkspaceInfoPath {
					workspace_id: workspace.id,
				})
				.headers(GetWorkspaceInfoRequestHeaders {
					authorization: BearerToken::from_str(&runner2.token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_status_ok();

	assert_eq!(
		StatusCode::ACCEPTED,
		regenerate(runner1.id).await.status_code(),
		"runner1 should be regenerable"
	);
}

/// Creating a runner and regenerating a runner's token take a human's
/// credential. A runner's own token is turned away, and so is a service
/// account that holds both permissions.
#[tokio::test]
async fn service_account_tokens_cannot_create_runners_or_regenerate_tokens() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![
				setup.get_permission_id(Permission::Runner(RunnerPermission::Create)),
				setup.get_permission_id(Permission::Runner(RunnerPermission::RegenerateToken)),
			],
		)
		.await;
	let service_account = setup
		.create_test_service_account(
			&admin.access_token,
			workspace.id,
			grants(role.id, &[workspace.id]),
		)
		.await;

	for token in [&runner.token, &service_account.token] {
		let create = setup
			.make_api_call(
				ApiRequest::<CreateRunnerRequest>::builder()
					.path(CreateRunnerPath {
						workspace_id: workspace.id,
					})
					.headers(CreateRunnerRequestHeaders {
						authorization: BearerToken::from_str(token).unwrap(),
						user_agent: TEST_USER_AGENT,
					})
					.body(CreateRunnerRequest {
						name: random_name(8),
					})
					.build(),
			)
			.await;
		assert_eq!(
			StatusCode::UNAUTHORIZED,
			create.status_code(),
			"a service account token should not create a runner"
		);

		let regenerate = setup
			.make_api_call(
				ApiRequest::<RegenerateRunnerTokenRequest>::builder()
					.path(RegenerateRunnerTokenPath {
						workspace_id: workspace.id,
						runner_id: runner.id,
					})
					.headers(RegenerateRunnerTokenRequestHeaders {
						authorization: BearerToken::from_str(token).unwrap(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
			)
			.await;
		assert_eq!(
			StatusCode::UNAUTHORIZED,
			regenerate.status_code(),
			"a service account token should not regenerate a runner's token"
		);
	}

	setup
		.make_api_call(
			ApiRequest::<GetWorkspaceInfoRequest>::builder()
				.path(GetWorkspaceInfoPath {
					workspace_id: workspace.id,
				})
				.headers(GetWorkspaceInfoRequestHeaders {
					authorization: BearerToken::from_str(&runner.token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_status_ok();
}

/// A runner can see itself, and no other runner.
#[tokio::test]
async fn runner_token_views_only_its_own_runner() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner1 = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let runner2 = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let get_runner_info = |runner_id| {
		setup.make_api_call(
			ApiRequest::<GetRunnerInfoRequest>::builder()
				.path(GetRunnerInfoPath {
					workspace_id: workspace.id,
					runner_id,
				})
				.headers(GetRunnerInfoRequestHeaders {
					authorization: BearerToken::from_str(&runner1.token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
	};

	let own = get_runner_info(runner1.id).await;
	assert!(
		own.status_code().is_success(),
		"a runner should view itself"
	);
	assert_eq!(
		runner1.id,
		own.json::<ApiSuccessResponseBody<GetRunnerInfoResponse>>()
			.response
			.runner
			.id
	);

	assert_eq!(
		StatusCode::UNAUTHORIZED,
		get_runner_info(runner2.id).await.status_code(),
		"a runner should not view another runner"
	);

	let response = setup
		.make_api_call(
			ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
				.path(ListRunnersForWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListRunnersForWorkspaceRequestHeaders {
					authorization: BearerToken::from_str(&runner1.token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!("1", response.header("x-total-count"));
	let body = response.json::<ApiSuccessResponseBody<ListRunnersForWorkspaceResponse>>();
	assert_eq!(1, body.response.runners.len());
	assert_eq!(runner1.id, body.response.runners[0].id);
}
