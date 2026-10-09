use std::{collections::BTreeMap, time::Duration};

use futures::SinkExt;
use models::{
	ApiSuccessResponseBody,
	api::workspace::{
		deployment::{
			DeploymentStatus,
			GetDeploymentInfoPath,
			GetDeploymentInfoRequest,
			GetDeploymentInfoRequestHeaders,
			GetDeploymentInfoResponse,
		},
		runner::*,
	},
	rbac::WorkspacePermission,
	utils::{BearerToken, ListResourceQuery, Uuid},
};
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest};

use crate::prelude::*;

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

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<AddRunnerToWorkspaceRequest>::builder()
				.path(AddRunnerToWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(AddRunnerToWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(AddRunnerToWorkspaceRequest {
					name: runner.name.clone(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"adding a runner with a taken name should fail"
	);
}

#[tokio::test]
async fn add_runner_invalid_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<AddRunnerToWorkspaceRequest>::builder()
				.path(AddRunnerToWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(AddRunnerToWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(AddRunnerToWorkspaceRequest {
					name: "!!!".to_string(),
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

	let dup = setup
		.make_web_dashboard_call(
			ApiRequest::<AddRunnerToWorkspaceRequest>::builder()
				.path(AddRunnerToWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(AddRunnerToWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(AddRunnerToWorkspaceRequest {
					name: runner.name.clone(),
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

	let recreate = setup
		.make_web_dashboard_call(
			ApiRequest::<AddRunnerToWorkspaceRequest>::builder()
				.path(AddRunnerToWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(AddRunnerToWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(AddRunnerToWorkspaceRequest {
					name: runner.name.clone(),
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
		let response = setup
			.make_web_dashboard_call(
				ApiRequest::<AddRunnerToWorkspaceRequest>::builder()
					.path(AddRunnerToWorkspacePath { workspace_id: ws })
					.headers(AddRunnerToWorkspaceRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(AddRunnerToWorkspaceRequest { name: name.clone() })
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
		let response = setup
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
			.await;
		assert_eq!("5", response.header("x-total-count"));
		pages.push(response.json::<ApiSuccessResponseBody<ListRunnersForWorkspaceResponse>>());
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

/// A non-zero page past the end of the result set is rejected as out of bounds
/// (PageOutOfBounds → 400) rather than returning an empty page.
#[tokio::test]
async fn list_runners_page_out_of_bounds() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
				.path(ListRunnersForWorkspacePath {
					workspace_id: workspace.id,
				})
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count: 10,
					page: 50,
					additional_query: (),
				})
				.headers(ListRunnersForWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(
		400,
		response.status_code().as_u16(),
		"a page past the end should be PageOutOfBounds (400)"
	);
}

/// The total count only covers the runners that match the search.
#[tokio::test]
async fn list_runners_search_counts_only_matches() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner1 = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
				.path(ListRunnersForWorkspacePath {
					workspace_id: workspace.id,
				})
				.query(ListResourceQuery {
					sort: None,
					search: RunnerSearchParams {
						name: Some(runner1.name.clone()),
						..Default::default()
					},
					count: 10,
					page: 0,
					additional_query: (),
				})
				.headers(ListRunnersForWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
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

/// Read a deployment's status through the API.
async fn deployment_status(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	deployment_id: Uuid,
) -> DeploymentStatus {
	setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id,
					deployment_id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetDeploymentInfoResponse>>()
		.response
		.deployment
		.status
		.clone()
}

#[tokio::test]
async fn runner_cannot_update_status_of_another_workspaces_deployment() {
	let setup = setup().await.expect("failed to setup test server");

	let victim = setup.create_test_user().await;
	let victim_workspace = setup.create_test_workspace(&victim.access_token).await;
	let victim_runner = setup
		.create_test_runner(&victim.access_token, victim_workspace.id)
		.await;
	let victim_deployment = setup
		.create_test_deployment(&victim.access_token, victim_workspace.id, victim_runner.id)
		.await;

	let attacker = setup.create_test_user().await;
	let attacker_workspace = setup.create_test_workspace(&attacker.access_token).await;
	let attacker_runner = setup
		.create_test_runner(&attacker.access_token, attacker_workspace.id)
		.await;
	let attacker_deployment = setup
		.create_test_deployment(
			&attacker.access_token,
			attacker_workspace.id,
			attacker_runner.id,
		)
		.await;

	// `deploying` → `running` is an allowed transition, so only the runner and
	// workspace checks stand between the attacker and the victim's deployment.
	setup
		.execute_sql(&format!(
			"UPDATE deployment SET status = 'deploying' WHERE id IN ('{}', '{}');",
			victim_deployment.id, attacker_deployment.id
		))
		.await;

	let victim_kv_writes = setup
		.cloudflare_kv_writes(&victim_deployment.id.to_string())
		.await;

	let token = setup
		.create_test_api_token(
			&attacker.access_token,
			BTreeMap::from([(attacker_workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await
		.token;
	let mut request = setup
		.api_ws_url(&format!(
			"/workspace/{}/runner/{}/stream",
			attacker_workspace.id, attacker_runner.id
		))
		.into_client_request()
		.unwrap();
	let headers = StreamRunnerDataForWorkspaceRequestHeaders {
		authorization: BearerToken::from_str(&token).unwrap(),
		user_agent: TEST_USER_AGENT,
	}
	.to_header_map();
	for (name, value) in headers.iter() {
		request.headers_mut().insert(name.clone(), value.clone());
	}
	let (mut runner_stream, _) = tokio_tungstenite::connect_async(request)
		.await
		.expect("the attacker's runner should connect");

	// The stream ignores everything until the runner sets its exposure type.
	// After that, messages are handled in order, so once the attacker's own
	// deployment is `running`, the update for the victim's has been handled too.
	let messages = [
		StreamRunnerDataForWorkspaceClientMsg::SetRunnerExposureType {
			exposure_type: RunnerExposureType::Private,
		},
		StreamRunnerDataForWorkspaceClientMsg::DeploymentStatusUpdated {
			id: victim_deployment.id,
			status: DeploymentStatus::Running,
		},
		StreamRunnerDataForWorkspaceClientMsg::DeploymentStatusUpdated {
			id: attacker_deployment.id,
			status: DeploymentStatus::Running,
		},
	];
	for message in messages {
		runner_stream
			.send(Message::Text(
				serde_json::to_string(&message).unwrap().into(),
			))
			.await
			.unwrap();
	}
	let mut attacker_status = DeploymentStatus::Deploying;
	for _ in 0..50 {
		attacker_status = deployment_status(
			&setup,
			&attacker.access_token,
			attacker_workspace.id,
			attacker_deployment.id,
		)
		.await;
		if attacker_status == DeploymentStatus::Running {
			break;
		}
		tokio::time::sleep(Duration::from_millis(100)).await;
	}
	assert_eq!(
		attacker_status,
		DeploymentStatus::Running,
		"the runner's own deployment should be updated"
	);

	assert_eq!(
		deployment_status(
			&setup,
			&victim.access_token,
			victim_workspace.id,
			victim_deployment.id,
		)
		.await,
		DeploymentStatus::Deploying,
		"another workspace's runner must not change the deployment's status"
	);
	assert_eq!(
		setup
			.cloudflare_kv_writes(&victim_deployment.id.to_string())
			.await,
		victim_kv_writes,
		"another workspace's runner must not rewrite the deployment's KV entry"
	);
}
