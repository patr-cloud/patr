use std::collections::{BTreeMap, BTreeSet};

use models::{
	api::{
		user::*,
		workspace::{
			GetWorkspaceInfoPath,
			GetWorkspaceInfoRequest,
			GetWorkspaceInfoRequestHeaders,
			container_registry::*,
			deployment::*,
			domain::*,
			managed_url::*,
			rbac::{role::*, user::*, *},
			runner::*,
			secret::*,
		},
	},
	rbac::{
		ContainerRegistryRepositoryPermission,
		DeploymentPermission,
		DomainPermission,
		ManagedURLPermission,
		Permission,
		RunnerPermission,
		SecretPermission,
		WorkspacePermission,
	},
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
			ApiRequest::<GetRunnerInfoRequest>::builder()
				.path(GetRunnerInfoPath {
					workspace_id: ws_id,
					runner_id: runner.id,
				})
				.headers(GetRunnerInfoRequestHeaders {
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
			ApiRequest::<GetRunnerInfoRequest>::builder()
				.path(GetRunnerInfoPath {
					workspace_id: ws_id,
					runner_id: runner.id,
				})
				.headers(GetRunnerInfoRequestHeaders {
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
			ApiRequest::<GetRunnerInfoRequest>::builder()
				.path(GetRunnerInfoPath {
					workspace_id: workspace.id,
					runner_id: runner2.id,
				})
				.headers(GetRunnerInfoRequestHeaders {
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

/// A runner's own token can neither create a runner nor regenerate a token.
#[tokio::test]
async fn runner_tokens_cannot_create_runners_or_regenerate_tokens() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;

	let create = setup
		.make_api_call(
			ApiRequest::<CreateRunnerRequest>::builder()
				.path(CreateRunnerPath {
					workspace_id: workspace.id,
				})
				.headers(CreateRunnerRequestHeaders {
					authorization: BearerToken::from_str(&runner.token).unwrap(),
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
		"a runner token should not create a runner"
	);

	let regenerate = setup
		.make_api_call(
			ApiRequest::<RegenerateRunnerTokenRequest>::builder()
				.path(RegenerateRunnerTokenPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(RegenerateRunnerTokenRequestHeaders {
					authorization: BearerToken::from_str(&runner.token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(
		StatusCode::UNAUTHORIZED,
		regenerate.status_code(),
		"a runner token should not regenerate a runner's token"
	);

	setup
		.make_api_call(
			ApiRequest::<GetRunnerInfoRequest>::builder()
				.path(GetRunnerInfoPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(GetRunnerInfoRequestHeaders {
					authorization: BearerToken::from_str(&runner.token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_status_ok();
}

/// A service account holding `runner::create` and `runner::regenerateToken`
/// manages runners like a user can, and one without them is turned away.
#[tokio::test]
async fn service_account_creates_runners_and_regenerates_tokens_with_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let manager = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![
				setup.get_permission_id(Permission::Runner(RunnerPermission::Create)),
				setup.get_permission_id(Permission::Runner(RunnerPermission::RegenerateToken)),
			],
		)
		.await;
	let viewer = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::Runner(RunnerPermission::View))],
		)
		.await;
	let manager = setup
		.create_test_service_account(
			&admin.access_token,
			workspace.id,
			grants(manager.id, &[workspace.id]),
		)
		.await;
	let viewer = setup
		.create_test_service_account(
			&admin.access_token,
			workspace.id,
			grants(viewer.id, &[workspace.id]),
		)
		.await;

	let create = |token: String| {
		let setup = &setup;
		async move {
			setup
				.make_api_call(
					ApiRequest::<CreateRunnerRequest>::builder()
						.path(CreateRunnerPath {
							workspace_id: workspace.id,
						})
						.headers(CreateRunnerRequestHeaders {
							authorization: BearerToken::from_str(&token).unwrap(),
							user_agent: TEST_USER_AGENT,
						})
						.body(CreateRunnerRequest {
							name: random_name(8),
						})
						.build(),
				)
				.await
		}
	};
	let regenerate = |token: String, runner_id: Uuid| {
		let setup = &setup;
		async move {
			setup
				.make_api_call(
					ApiRequest::<RegenerateRunnerTokenRequest>::builder()
						.path(RegenerateRunnerTokenPath {
							workspace_id: workspace.id,
							runner_id,
						})
						.headers(RegenerateRunnerTokenRequestHeaders {
							authorization: BearerToken::from_str(&token).unwrap(),
							user_agent: TEST_USER_AGENT,
						})
						.build(),
				)
				.await
		}
	};
	let runner_info = |token: String, runner_id: Uuid| {
		let setup = &setup;
		async move {
			setup
				.make_api_call(
					ApiRequest::<GetRunnerInfoRequest>::builder()
						.path(GetRunnerInfoPath {
							workspace_id: workspace.id,
							runner_id,
						})
						.headers(GetRunnerInfoRequestHeaders {
							authorization: BearerToken::from_str(&token).unwrap(),
							user_agent: TEST_USER_AGENT,
						})
						.build(),
				)
				.await
		}
	};

	let created = create(manager.token.clone())
		.await
		.json::<ApiSuccessResponseBody<CreateRunnerResponse>>()
		.response;
	runner_info(created.token.clone(), created.id.id)
		.await
		.assert_status_ok();

	let regenerated = regenerate(manager.token.clone(), created.id.id).await;
	assert_eq!(
		StatusCode::ACCEPTED,
		regenerated.status_code(),
		"a service account with runner::regenerateToken should regenerate"
	);
	let regenerated = regenerated
		.json::<ApiSuccessResponseBody<RegenerateRunnerTokenResponse>>()
		.response;
	assert_eq!(
		StatusCode::UNAUTHORIZED,
		runner_info(created.token, created.id.id)
			.await
			.status_code(),
		"the runner's old token should stop working"
	);
	runner_info(regenerated.token, created.id.id)
		.await
		.assert_status_ok();

	assert_eq!(
		StatusCode::UNAUTHORIZED,
		create(viewer.token.clone()).await.status_code(),
		"a service account without runner::create should not create a runner"
	);
	assert_eq!(
		StatusCode::UNAUTHORIZED,
		regenerate(viewer.token, created.id.id).await.status_code(),
		"a service account without runner::regenerateToken should not regenerate"
	);
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
}

/// A runner's token reaches only the routes that take runners. Even in its own
/// workspace it can't read the workspace's details, create a role or change a
/// member's roles, and it can't mint an API token.
#[tokio::test]
async fn runner_token_is_refused_by_routes_without_runner() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let token = BearerToken::from_str(&runner.token).unwrap();

	assert_eq!(
		StatusCode::UNAUTHORIZED,
		setup
			.make_api_call(
				ApiRequest::<GetWorkspaceInfoRequest>::builder()
					.path(GetWorkspaceInfoPath {
						workspace_id: workspace.id,
					})
					.headers(GetWorkspaceInfoRequestHeaders {
						authorization: token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
			)
			.await
			.status_code(),
		"a runner should not read its workspace's details"
	);

	assert_eq!(
		StatusCode::UNAUTHORIZED,
		setup
			.make_api_call(
				ApiRequest::<CreateNewRoleRequest>::builder()
					.path(CreateNewRolePath {
						workspace_id: workspace.id,
					})
					.headers(CreateNewRoleRequestHeaders {
						authorization: token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(CreateNewRoleRequest {
						role: Role {
							name: random_name(8),
							description: "created by a runner".to_string(),
							is_immutable: false,
						},
						permissions: BTreeSet::from([
							setup.get_permission_id(Permission::ViewRoles)
						]),
					})
					.build(),
			)
			.await
			.status_code(),
		"a runner should not create a role"
	);

	assert_eq!(
		StatusCode::UNAUTHORIZED,
		setup
			.make_api_call(
				ApiRequest::<UpdateUserRolesInWorkspaceRequest>::builder()
					.path(UpdateUserRolesInWorkspacePath {
						workspace_id: workspace.id,
						user_id: admin.user_id,
					})
					.headers(UpdateUserRolesInWorkspaceRequestHeaders {
						authorization: token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(UpdateUserRolesInWorkspaceRequest { roles: Vec::new() })
					.build(),
			)
			.await
			.status_code(),
		"a runner should not change a member's roles"
	);

	// Only web logins create API tokens, so the route is mounted on the
	// dashboard host alone, where anything that isn't a JWT is malformed.
	assert_eq!(
		StatusCode::BAD_REQUEST,
		setup
			.make_web_dashboard_call(
				ApiRequest::<CreateApiTokenRequest>::builder()
					.headers(CreateApiTokenRequestHeaders {
						authorization: token,
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
							allowed_ips: None,
							created: time::OffsetDateTime::now_utc(),
						},
					})
					.build(),
			)
			.await
			.status_code(),
		"a runner should not mint an API token"
	);
}

/// A runner reads what it needs to run deployments across its own workspace,
/// and its permissions are exactly the fixed set every runner gets.
#[tokio::test]
async fn runner_token_reads_its_own_workspace() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;
	let domain = setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;
	let managed_url_id = setup
		.create_test_managed_url(&admin.access_token, workspace.id, domain.id)
		.await;
	let secret = setup
		.create_test_secret(&admin.access_token, workspace.id)
		.await;
	let repo = setup
		.create_test_container_repo(&admin.access_token, workspace.id)
		.await;
	let token = BearerToken::from_str(&runner.token).unwrap();

	let deployments = setup
		.make_api_call(
			ApiRequest::<ListDeploymentRequest>::builder()
				.path(ListDeploymentPath {
					workspace_id: workspace.id,
				})
				.headers(ListDeploymentRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListDeploymentResponse>>()
		.response
		.deployments;
	assert_eq!(
		vec![deployment.id],
		deployments
			.iter()
			.map(|deployment| deployment.id)
			.collect::<Vec<_>>()
	);

	setup
		.make_api_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_status_ok();

	let urls = setup
		.make_api_call(
			ApiRequest::<ListManagedURLRequest>::builder()
				.path(ListManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(ListManagedURLRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListManagedURLResponse>>()
		.response
		.urls;
	assert_eq!(
		vec![managed_url_id],
		urls.iter().map(|url| url.id).collect::<Vec<_>>()
	);

	setup
		.make_api_call(
			ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
				.path(GetDomainInfoInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(GetDomainInfoInWorkspaceRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_status_ok();

	setup
		.make_api_call(
			ApiRequest::<GetContainerRepositoryInfoRequest>::builder()
				.path(GetContainerRepositoryInfoPath {
					workspace_id: workspace.id,
					repository_id: repo.id,
				})
				.headers(GetContainerRepositoryInfoRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_status_ok();

	let secrets = setup
		.make_api_call(
			ApiRequest::<ListSecretsForWorkspaceRequest>::builder()
				.path(ListSecretsForWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListSecretsForWorkspaceRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListSecretsForWorkspaceResponse>>()
		.response
		.secrets;
	assert_eq!(
		vec![secret.id],
		secrets.iter().map(|secret| secret.id).collect::<Vec<_>>()
	);

	let permissions = setup
		.make_api_call(
			ApiRequest::<GetCurrentPermissionsRequest>::builder()
				.path(GetCurrentPermissionsPath {
					workspace_id: workspace.id,
				})
				.headers(GetCurrentPermissionsRequestHeaders {
					authorization: token,
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetCurrentPermissionsResponse>>()
		.response
		.permissions;
	assert_eq!(
		WorkspacePermission::Member {
			permissions: [
				Permission::ContainerRegistryRepository(
					ContainerRegistryRepositoryPermission::View
				),
				Permission::ContainerRegistryRepository(
					ContainerRegistryRepositoryPermission::Pull
				),
				Permission::Deployment(DeploymentPermission::View),
				Permission::Domain(DomainPermission::View),
				Permission::ManagedURL(ManagedURLPermission::View),
				Permission::Secret(SecretPermission::View),
			]
			.into_iter()
			.map(|permission| {
				(
					setup.get_permission_id(permission),
					BTreeSet::from([workspace.id]),
				)
			})
			.chain(
				[
					Permission::Runner(RunnerPermission::View),
					Permission::Runner(RunnerPermission::Edit),
				]
				.map(|permission| {
					(
						setup.get_permission_id(permission),
						BTreeSet::from([runner.id]),
					)
				}),
			)
			.collect(),
		},
		permissions,
		"a runner holds exactly the fixed set"
	);
}

/// A runner reads nothing in a workspace it doesn't belong to, even one with
/// the same owner.
#[tokio::test]
async fn runner_token_is_refused_in_another_workspace() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let other_workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let other_runner = setup
		.create_test_runner(&admin.access_token, other_workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&admin.access_token, other_workspace.id, other_runner.id)
		.await;
	let domain = setup
		.create_test_domain(&admin.access_token, other_workspace.id)
		.await;
	let repo = setup
		.create_test_container_repo(&admin.access_token, other_workspace.id)
		.await;
	let token = BearerToken::from_str(&runner.token).unwrap();

	let statuses = [
		setup
			.make_api_call(
				ApiRequest::<ListDeploymentRequest>::builder()
					.path(ListDeploymentPath {
						workspace_id: other_workspace.id,
					})
					.headers(ListDeploymentRequestHeaders {
						authorization: token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
			)
			.await
			.status_code(),
		setup
			.make_api_call(
				ApiRequest::<GetDeploymentInfoRequest>::builder()
					.path(GetDeploymentInfoPath {
						workspace_id: other_workspace.id,
						deployment_id: deployment.id,
					})
					.headers(GetDeploymentInfoRequestHeaders {
						authorization: token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
			)
			.await
			.status_code(),
		setup
			.make_api_call(
				ApiRequest::<ListManagedURLRequest>::builder()
					.path(ListManagedURLPath {
						workspace_id: other_workspace.id,
					})
					.headers(ListManagedURLRequestHeaders {
						authorization: token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
			)
			.await
			.status_code(),
		setup
			.make_api_call(
				ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
					.path(GetDomainInfoInWorkspacePath {
						workspace_id: other_workspace.id,
						domain_id: domain.id,
					})
					.headers(GetDomainInfoInWorkspaceRequestHeaders {
						authorization: token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
			)
			.await
			.status_code(),
		setup
			.make_api_call(
				ApiRequest::<GetContainerRepositoryInfoRequest>::builder()
					.path(GetContainerRepositoryInfoPath {
						workspace_id: other_workspace.id,
						repository_id: repo.id,
					})
					.headers(GetContainerRepositoryInfoRequestHeaders {
						authorization: token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
			)
			.await
			.status_code(),
		setup
			.make_api_call(
				ApiRequest::<ListSecretsForWorkspaceRequest>::builder()
					.path(ListSecretsForWorkspacePath {
						workspace_id: other_workspace.id,
					})
					.headers(ListSecretsForWorkspaceRequestHeaders {
						authorization: token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
			)
			.await
			.status_code(),
		setup
			.make_api_call(
				ApiRequest::<GetCurrentPermissionsRequest>::builder()
					.path(GetCurrentPermissionsPath {
						workspace_id: other_workspace.id,
					})
					.headers(GetCurrentPermissionsRequestHeaders {
						authorization: token,
						user_agent: TEST_USER_AGENT,
					})
					.build(),
			)
			.await
			.status_code(),
	];

	assert_eq!(
		[StatusCode::UNAUTHORIZED; 7],
		statuses,
		"a runner should read nothing in another workspace"
	);
}

/// `RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID` gives a runner exactly what its
/// token holds, for every permission there is. The workspace-wide reads cover
/// every live resource in its own workspace and none in another, viewing
/// covers the runner itself, and everything else covers nothing. A deleted
/// runner gets nothing at all.
#[tokio::test]
async fn runner_resources_with_permission_match_its_token() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let other_workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	// Another runner to host the deployment, so the runner under test can be
	// deleted at the end.
	let sibling_runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	setup
		.create_test_deployment(&admin.access_token, workspace.id, sibling_runner.id)
		.await;
	setup
		.create_test_secret(&admin.access_token, workspace.id)
		.await;
	setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;
	setup
		.create_test_container_repo(&admin.access_token, workspace.id)
		.await;
	let deleted_secret = setup
		.create_test_secret(&admin.access_token, workspace.id)
		.await;
	setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteSecretRequest>::builder()
				.path(DeleteSecretPath {
					workspace_id: workspace.id,
					secret_id: deleted_secret.id,
				})
				.headers(DeleteSecretRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_status(StatusCode::ACCEPTED);
	setup
		.create_test_runner(&admin.access_token, other_workspace.id)
		.await;
	setup
		.create_test_secret(&admin.access_token, other_workspace.id)
		.await;
	setup
		.create_test_container_repo(&admin.access_token, other_workspace.id)
		.await;

	let resources_with_permission = async |permission: Permission| {
		sqlx::query_scalar::<_, Uuid>(
			"SELECT id FROM RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID($1, $2)",
		)
		.bind(runner.id)
		.bind(permission.to_string())
		.fetch_all(setup.database())
		.await
		.expect("RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID")
		.into_iter()
		.collect::<BTreeSet<_>>()
	};

	let live_resources = sqlx::query_scalar::<_, Uuid>(
		"SELECT id FROM resource WHERE workspace_id = $1 AND deleted IS NULL",
	)
	.bind(workspace.id)
	.fetch_all(setup.database())
	.await
	.expect("live resources")
	.into_iter()
	.collect::<BTreeSet<_>>();
	assert!(!live_resources.contains(&deleted_secret.id));

	let WorkspacePermission::Member {
		permissions: token_permissions,
	} = setup
		.make_api_call(
			ApiRequest::<GetCurrentPermissionsRequest>::builder()
				.path(GetCurrentPermissionsPath {
					workspace_id: workspace.id,
				})
				.headers(GetCurrentPermissionsRequestHeaders {
					authorization: BearerToken::from_str(&runner.token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetCurrentPermissionsResponse>>()
		.response
		.permissions
	else {
		panic!("a runner is never a super admin");
	};

	for permission in Permission::list_all() {
		let resources = resources_with_permission(permission).await;

		// Pinned to the token's map: every live resource a scope covers.
		let scopes = token_permissions
			.get(&setup.get_permission_id(permission))
			.cloned()
			.unwrap_or_default();
		assert_eq!(
			live_resources
				.iter()
				.filter(|resource_id| {
					scopes.contains(resource_id) || scopes.contains(&workspace.id)
				})
				.copied()
				.collect::<BTreeSet<_>>(),
			resources,
			"`{permission}` disagrees with the runner's token"
		);

		// And to the fixed set itself, so a change to both sides together is
		// still caught.
		match permission {
			Permission::ContainerRegistryRepository(
				ContainerRegistryRepositoryPermission::View |
				ContainerRegistryRepositoryPermission::Pull,
			) |
			Permission::Deployment(DeploymentPermission::View) |
			Permission::Domain(DomainPermission::View) |
			Permission::ManagedURL(ManagedURLPermission::View) |
			Permission::Secret(SecretPermission::View) => {
				assert_eq!(live_resources, resources, "`{permission}`");
			}
			Permission::Runner(RunnerPermission::View | RunnerPermission::Edit) => {
				assert_eq!(BTreeSet::from([runner.id]), resources, "`{permission}`");
			}
			_ => assert!(resources.is_empty(), "`{permission}` should cover nothing"),
		}
	}

	setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteRunnerRequest>::builder()
				.path(DeleteRunnerPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(DeleteRunnerRequestHeaders {
					authorization: admin.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_status(StatusCode::ACCEPTED);

	for permission in Permission::list_all() {
		assert!(
			resources_with_permission(permission).await.is_empty(),
			"a deleted runner should get nothing for `{permission}`"
		);
	}
}

/// A runner's token acts as that runner only: it can't view another runner,
/// fetch another runner's ingress token or open another runner's data stream.
/// A service account can do neither of the last two for any runner.
#[tokio::test]
async fn runner_token_cannot_act_as_another_runner() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let other_runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let service_account = setup
		.create_test_service_account(&admin.access_token, workspace.id, Vec::new())
		.await;
	let ingress_token = |token: &str, runner_id| {
		setup.make_api_call(
			ApiRequest::<GetIngressTokenForRunnerRequest>::builder()
				.path(GetIngressTokenForRunnerPath {
					workspace_id: workspace.id,
					runner_id,
				})
				.headers(GetIngressTokenForRunnerRequestHeaders {
					authorization: BearerToken::from_str(token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
	};
	let stream = |token: &str, runner_id| {
		setup.connect_api_websocket(
			StreamRunnerDataForWorkspacePath {
				workspace_id: workspace.id,
				runner_id,
			},
			StreamRunnerDataForWorkspaceRequestHeaders {
				authorization: BearerToken::from_str(token).unwrap(),
				user_agent: TEST_USER_AGENT,
			},
		)
	};

	assert_eq!(
		StatusCode::UNAUTHORIZED,
		setup
			.make_api_call(
				ApiRequest::<GetRunnerInfoRequest>::builder()
					.path(GetRunnerInfoPath {
						workspace_id: workspace.id,
						runner_id: other_runner.id,
					})
					.headers(GetRunnerInfoRequestHeaders {
						authorization: BearerToken::from_str(&runner.token).unwrap(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
			)
			.await
			.status_code(),
		"a runner should not view another runner"
	);

	assert_eq!(
		StatusCode::UNAUTHORIZED,
		ingress_token(&runner.token, other_runner.id)
			.await
			.status_code(),
		"a runner should not fetch another runner's ingress token"
	);
	assert_eq!(
		StatusCode::UNAUTHORIZED,
		ingress_token(&service_account.token, runner.id)
			.await
			.status_code(),
		"a service account should not fetch a runner's ingress token"
	);
	assert_eq!(
		"",
		sqlx::query_scalar::<_, String>("SELECT cloudflare_tunnel_id FROM runner WHERE id = $1")
			.bind(other_runner.id)
			.fetch_one(setup.database())
			.await
			.expect("tunnel query"),
		"a refused fetch shouldn't create the runner's tunnel"
	);

	assert_eq!(
		Some(StatusCode::UNAUTHORIZED),
		stream(&runner.token, other_runner.id).await.err(),
		"a runner should not open another runner's stream"
	);
	assert_eq!(
		Some(StatusCode::UNAUTHORIZED),
		stream(&service_account.token, runner.id).await.err(),
		"a service account should not open a runner's stream"
	);
}
