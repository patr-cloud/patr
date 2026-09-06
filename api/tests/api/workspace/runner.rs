use std::collections::BTreeMap;

use models::{
	ApiSuccessResponseBody,
	api::workspace::runner::*,
	rbac::WorkspacePermission,
	utils::{ListResourceQuery, Uuid},
};

use crate::prelude::*;

/// Open a fresh consent link as the CLI would (via an API token), returning the
/// link details and the API bearer needed to later verify it.
async fn open_runner_link(
	setup: &TestSetup,
	admin: &BearerToken,
	workspace_id: Uuid,
) -> (CreateRunnerLinkResponse, BearerToken) {
	let api_token = setup
		.create_test_api_token(
			admin,
			BTreeMap::from([(workspace_id, WorkspacePermission::SuperAdmin)]),
		)
		.await;
	let api_bearer = BearerToken::from_str(&api_token.token).unwrap();

	let link = setup
		.make_api_call(
			ApiRequest::<CreateRunnerLinkRequest>::builder()
				.path(CreateRunnerLinkPath { workspace_id })
				.headers(CreateRunnerLinkRequestHeaders {
					authorization: api_bearer.clone(),
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

	(link, api_bearer)
}

#[tokio::test]
async fn add_runner_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	assert!(!runner.name.is_empty());
}

#[tokio::test]
async fn list_runners_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let _runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
				.path(ListRunnersForWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListRunnersForWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListRunnersForWorkspaceResponse>>();

	assert_eq!(1, response.response.runners.len());
}

#[tokio::test]
async fn list_runners_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
				.path(ListRunnersForWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListRunnersForWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListRunnersForWorkspaceResponse>>();

	assert!(response.response.runners.is_empty());
}

#[tokio::test]
async fn get_runner_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetRunnerInfoRequest>::builder()
				.path(GetRunnerInfoPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(GetRunnerInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetRunnerInfoResponse>>();

	assert_eq!(runner.name, response.response.runner.name);
}

#[tokio::test]
async fn get_runner_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetRunnerInfoRequest>::builder()
				.path(GetRunnerInfoPath {
					workspace_id: workspace.id,
					runner_id: Uuid::nil(),
				})
				.headers(GetRunnerInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for nonexistent runner"
	);
}

#[tokio::test]
async fn get_ingress_token_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetIngressTokenForRunnerRequest>::builder()
				.path(GetIngressTokenForRunnerPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(GetIngressTokenForRunnerRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetIngressTokenForRunnerResponse>>();

	assert!(
		!response.response.token.is_empty(),
		"ingress token should not be empty"
	);
}

#[tokio::test]
async fn remove_runner_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteRunnerRequest>::builder()
				.path(DeleteRunnerPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(DeleteRunnerRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteRunnerResponse));

	// Verify it's gone
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetRunnerInfoRequest>::builder()
				.path(GetRunnerInfoPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(GetRunnerInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"deleted runner should not be found"
	);
}

#[tokio::test]
async fn remove_runner_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteRunnerRequest>::builder()
				.path(DeleteRunnerPath {
					workspace_id: workspace.id,
					runner_id: Uuid::nil(),
				})
				.headers(DeleteRunnerRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for nonexistent runner"
	);
}

#[tokio::test]
async fn add_runner_duplicate_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	// The name uniqueness check now lives in the approve step of the link flow.
	let (link, _) = open_runner_link(&setup, &user.access_token, workspace.id).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ApproveRunnerLinkRequest>::builder()
				.path(ApproveRunnerLinkPath {
					workspace_id: workspace.id,
					user_code: link.user_code,
				})
				.headers(ApproveRunnerLinkRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ApproveRunnerLinkRequest {
					runner_name: runner.name.clone(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"approving a link with a taken runner name should fail"
	);
}

#[tokio::test]
async fn add_runner_invalid_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let (link, _) = open_runner_link(&setup, &user.access_token, workspace.id).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ApproveRunnerLinkRequest>::builder()
				.path(ApproveRunnerLinkPath {
					workspace_id: workspace.id,
					user_code: link.user_code,
				})
				.headers(ApproveRunnerLinkRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ApproveRunnerLinkRequest {
					runner_name: "!!!".to_string(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"runner name failing RESOURCE_NAME_REGEX should be rejected"
	);
}

#[tokio::test]
async fn get_ingress_token_nonexistent_runner() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetIngressTokenForRunnerRequest>::builder()
				.path(GetIngressTokenForRunnerPath {
					workspace_id: workspace.id,
					runner_id: Uuid::nil(),
				})
				.headers(GetIngressTokenForRunnerRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"ingress token for nonexistent runner should fail"
	);
}

#[tokio::test]
async fn runner_cross_workspace() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace_a = setup.create_test_workspace(&user.access_token).await;
	let workspace_b = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace_a.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetRunnerInfoRequest>::builder()
				.path(GetRunnerInfoPath {
					workspace_id: workspace_b.id,
					runner_id: runner.id,
				})
				.headers(GetRunnerInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"runner in workspace A should not be accessible from workspace B"
	);
}

/// Deleting a runner referenced by a (non-deleted) deployment is blocked with
/// ResourceInUse (422); it succeeds once the deployment is gone.
#[tokio::test]
async fn remove_runner_in_use_by_deployment() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let blocked = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteRunnerRequest>::builder()
				.path(DeleteRunnerPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(DeleteRunnerRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(
		422,
		blocked.status_code().as_u16(),
		"deleting a runner in use by a deployment should be ResourceInUse (422)"
	);

	// Remove the deployment, then the runner deletes cleanly.
	setup
		.make_web_dashboard_call(
			ApiRequest::<models::api::workspace::deployment::DeleteDeploymentRequest>::builder()
				.path(models::api::workspace::deployment::DeleteDeploymentPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(
					models::api::workspace::deployment::DeleteDeploymentRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					},
				)
				.build(),
		)
		.await;

	let allowed = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteRunnerRequest>::builder()
				.path(DeleteRunnerPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(DeleteRunnerRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		allowed.status_code().is_success(),
		"deleting the runner should succeed once the deployment is gone, got {}",
		allowed.status_code()
	);
}

/// A duplicate active name is rejected with 409, but the name becomes available
/// again once the runner is deleted (partial unique index WHERE deleted IS
/// NULL).
#[tokio::test]
async fn add_runner_reusable_after_delete() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let (dup_link, _) = open_runner_link(&setup, &user.access_token, workspace.id).await;
	let dup = setup
		.make_web_dashboard_call(
			ApiRequest::<ApproveRunnerLinkRequest>::builder()
				.path(ApproveRunnerLinkPath {
					workspace_id: workspace.id,
					user_code: dup_link.user_code,
				})
				.headers(ApproveRunnerLinkRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ApproveRunnerLinkRequest {
					runner_name: runner.name.clone(),
				})
				.build(),
		)
		.await;
	assert_eq!(
		409,
		dup.status_code().as_u16(),
		"duplicate runner name should be ResourceAlreadyExists (409)"
	);

	setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteRunnerRequest>::builder()
				.path(DeleteRunnerPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(DeleteRunnerRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteRunnerResponse));

	let (reuse_link, _) = open_runner_link(&setup, &user.access_token, workspace.id).await;
	let recreate = setup
		.make_web_dashboard_call(
			ApiRequest::<ApproveRunnerLinkRequest>::builder()
				.path(ApproveRunnerLinkPath {
					workspace_id: workspace.id,
					user_code: reuse_link.user_code,
				})
				.headers(ApproveRunnerLinkRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(ApproveRunnerLinkRequest {
					runner_name: runner.name.clone(),
				})
				.build(),
		)
		.await;
	assert!(
		recreate.status_code().is_success(),
		"the name should be reusable after delete, got {}",
		recreate.status_code()
	);
}

/// The same runner name is allowed in two different workspaces.
#[tokio::test]
async fn add_runner_same_name_across_workspaces() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace_a = setup.create_test_workspace(&user.access_token).await;
	let workspace_b = setup.create_test_workspace(&user.access_token).await;
	let name = random_name(8);

	for ws in [workspace_a.id, workspace_b.id] {
		let (link, _) = open_runner_link(&setup, &user.access_token, ws).await;
		let response = setup
			.make_web_dashboard_call(
				ApiRequest::<ApproveRunnerLinkRequest>::builder()
					.path(ApproveRunnerLinkPath {
						workspace_id: ws,
						user_code: link.user_code,
					})
					.headers(ApproveRunnerLinkRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(ApproveRunnerLinkRequest {
						runner_name: name.clone(),
					})
					.build(),
			)
			.await;
		assert!(
			response.status_code().is_success(),
			"same name should be allowed in each workspace, got {}",
			response.status_code()
		);
	}
}

/// The list is ordered created descending (newest first).
#[tokio::test]
async fn list_runners_ordered_created_desc() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let mut names = Vec::new();
	for _ in 0..3 {
		let runner = setup
			.create_test_runner(&user.access_token, workspace.id)
			.await;
		names.push(runner.name);
	}

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
				.path(ListRunnersForWorkspacePath {
					workspace_id: workspace.id,
				})
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count: 100,
					page: 0,
					additional_query: (),
				})
				.headers(ListRunnersForWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListRunnersForWorkspaceResponse>>();

	let listed: Vec<String> = response
		.response
		.runners
		.iter()
		.map(|r| r.name.clone())
		.collect();
	names.reverse();
	assert_eq!(names, listed, "runners should be ordered created DESC");
}

/// page/count slice the runner list and pages don't overlap.
#[tokio::test]
async fn list_runners_pagination() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	for _ in 0..5 {
		setup
			.create_test_runner(&user.access_token, workspace.id)
			.await;
	}

	let mut pages = Vec::new();
	for page in 0..2usize {
		pages.push(
			setup
				.make_web_dashboard_call(
					ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
						.path(ListRunnersForWorkspacePath {
							workspace_id: workspace.id,
						})
						.query(ListResourceQuery {
							sort: None,
							search: Default::default(),
							count: 2,
							page,
							additional_query: (),
						})
						.headers(ListRunnersForWorkspaceRequestHeaders {
							authorization: user.access_token.clone(),
							user_agent: TEST_USER_AGENT,
						})
						.build(),
				)
				.await
				.json::<ApiSuccessResponseBody<ListRunnersForWorkspaceResponse>>(),
		);
	}
	assert_eq!(2, pages[0].response.runners.len());
	assert_eq!(2, pages[1].response.runners.len());

	let ids: std::collections::BTreeSet<Uuid> = pages[0]
		.response
		.runners
		.iter()
		.chain(pages[1].response.runners.iter())
		.map(|r| r.id)
		.collect();
	assert_eq!(4, ids.len(), "the two pages should not overlap");
}

/// Deleting an already-deleted runner hits the soft-deleted resource and is
/// denied by the authorizer (401 — anti-enumeration).
#[tokio::test]
async fn remove_runner_already_deleted() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteRunnerRequest>::builder()
				.path(DeleteRunnerPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(DeleteRunnerRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteRunnerResponse));

	let second = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteRunnerRequest>::builder()
				.path(DeleteRunnerPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(DeleteRunnerRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(
		401,
		second.status_code().as_u16(),
		"deleting an already-deleted runner should 401 (anti-enumeration)"
	);
}

#[tokio::test]
async fn runner_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
				.path(ListRunnersForWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListRunnersForWorkspaceRequestHeaders {
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
async fn reconnect_runner_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	// Open a fresh link and reconnect the existing runner onto it.
	let (link, api_bearer) = open_runner_link(&setup, &user.access_token, workspace.id).await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<ReconnectRunnerLinkRequest>::builder()
				.path(ReconnectRunnerLinkPath {
					workspace_id: workspace.id,
					user_code: link.user_code.clone(),
					runner_id: runner.id,
				})
				.headers(ReconnectRunnerLinkRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(ReconnectRunnerLinkResponse));

	// The CLI claims the rotated credentials — same runner, a different token.
	let verify = setup
		.make_api_call(
			ApiRequest::<VerifyRunnerLinkRequest>::builder()
				.path(VerifyRunnerLinkPath {
					workspace_id: workspace.id,
				})
				.headers(VerifyRunnerLinkRequestHeaders {
					authorization: api_bearer,
					user_agent: TEST_USER_AGENT,
				})
				.body(VerifyRunnerLinkRequest {
					user_code: link.user_code,
					device_code: link.device_code,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<VerifyRunnerLinkResponse>>()
		.response;

	match verify.result {
		VerifyRunnerLinkResult::Approved {
			runner_id, token, ..
		} => {
			assert_eq!(
				runner_id, runner.id,
				"reconnect must target the same runner"
			);
			assert_ne!(token, runner.token, "reconnect must rotate the token");
		}
		VerifyRunnerLinkResult::Pending => panic!("link should be approved after reconnect"),
	}
}

#[tokio::test]
async fn reconnect_nonexistent_runner() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let (link, _api_bearer) = open_runner_link(&setup, &user.access_token, workspace.id).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ReconnectRunnerLinkRequest>::builder()
				.path(ReconnectRunnerLinkPath {
					workspace_id: workspace.id,
					user_code: link.user_code,
					runner_id: Uuid::nil(),
				})
				.headers(ReconnectRunnerLinkRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"reconnecting a nonexistent runner should be rejected"
	);
}

#[tokio::test]
async fn reconnect_twice_is_rejected() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let (link, _api_bearer) = open_runner_link(&setup, &user.access_token, workspace.id).await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<ReconnectRunnerLinkRequest>::builder()
				.path(ReconnectRunnerLinkPath {
					workspace_id: workspace.id,
					user_code: link.user_code.clone(),
					runner_id: runner.id,
				})
				.headers(ReconnectRunnerLinkRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(ReconnectRunnerLinkResponse));

	// The link is single-use — a second reconnect on the same code is rejected.
	let second = setup
		.make_web_dashboard_call(
			ApiRequest::<ReconnectRunnerLinkRequest>::builder()
				.path(ReconnectRunnerLinkPath {
					workspace_id: workspace.id,
					user_code: link.user_code,
					runner_id: runner.id,
				})
				.headers(ReconnectRunnerLinkRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		second.status_code().is_client_error(),
		"a link that's already been claimed should not reconnect again"
	);
}
