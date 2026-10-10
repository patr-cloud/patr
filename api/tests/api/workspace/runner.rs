use std::{collections::BTreeMap, time::Duration};

use api::redis::keys;
use base64::prelude::*;
use futures::{SinkExt as _, StreamExt as _};
use models::{
	ApiSuccessResponseBody,
	api::workspace::{
		GetWorkspaceInfoPath,
		GetWorkspaceInfoRequest,
		GetWorkspaceInfoRequestHeaders,
		deployment::*,
		runner::*,
	},
	rbac::WorkspacePermission,
	utils::{BearerToken, ListResourceQuery, Uuid},
};
use rustis::commands::GenericCommands as _;
use sha2::{Digest as _, Sha256};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};

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
	assert!(
		runner.token.starts_with("patr_sa_"),
		"a runner's token should be a service account token"
	);

	// The token authenticates as the runner's service account
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
		.make_api_call(
			ApiRequest::<GetIngressTokenForRunnerRequest>::builder()
				.path(GetIngressTokenForRunnerPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(GetIngressTokenForRunnerRequestHeaders {
					authorization: BearerToken::from_str(&runner.token).unwrap(),
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
			ApiRequest::<CreateRunnerRequest>::builder()
				.path(CreateRunnerPath {
					workspace_id: workspace.id,
				})
				.headers(CreateRunnerRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateRunnerRequest {
					name: runner.name.clone(),
				})
				.build(),
		)
		.await;

	assert_eq!(
		409,
		response.status_code().as_u16(),
		"a taken runner name should be ResourceAlreadyExists (409)"
	);
}

#[tokio::test]
async fn add_runner_invalid_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateRunnerRequest>::builder()
				.path(CreateRunnerPath {
					workspace_id: workspace.id,
				})
				.headers(CreateRunnerRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateRunnerRequest {
					name: "!!!".to_string(),
				})
				.build(),
		)
		.await;

	assert_eq!(
		400,
		response.status_code().as_u16(),
		"runner name failing RESOURCE_NAME_REGEX should be rejected with 400"
	);
}

#[tokio::test]
async fn get_ingress_token_nonexistent_runner() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<GetIngressTokenForRunnerRequest>::builder()
				.path(GetIngressTokenForRunnerPath {
					workspace_id: workspace.id,
					runner_id: Uuid::nil(),
				})
				.headers(GetIngressTokenForRunnerRequestHeaders {
					authorization: BearerToken::from_str(&runner.token).unwrap(),
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

	let create = || {
		setup.make_web_dashboard_call(
			ApiRequest::<CreateRunnerRequest>::builder()
				.path(CreateRunnerPath {
					workspace_id: workspace.id,
				})
				.headers(CreateRunnerRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateRunnerRequest {
					name: runner.name.clone(),
				})
				.build(),
		)
	};

	let dup = create().await;
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

	let recreate = create().await;
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
				ApiRequest::<CreateRunnerRequest>::builder()
					.path(CreateRunnerPath { workspace_id: ws })
					.headers(CreateRunnerRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(CreateRunnerRequest { name: name.clone() })
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

	let mut runner_stream = connect_runner(
		&setup,
		attacker_workspace.id,
		attacker_runner.id,
		&attacker_runner.token,
	)
	.await;

	// Messages are handled in order, so once the attacker's own deployment is
	// `running`, the update for the victim's has been handled too.
	let messages = [
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

/// Regenerating a runner's token rejects the old one at once, even once
/// cached, and drops the runner's connection lock so a stream on the old token
/// closes at its next ping.
#[tokio::test]
async fn regenerated_runner_old_token_is_rejected_immediately() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let get_workspace_info = |token: &str| {
		setup.make_api_call(
			ApiRequest::<GetWorkspaceInfoRequest>::builder()
				.path(GetWorkspaceInfoPath {
					workspace_id: workspace.id,
				})
				.headers(GetWorkspaceInfoRequestHeaders {
					authorization: BearerToken::from_str(token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
	};

	assert!(
		get_workspace_info(&runner.token)
			.await
			.status_code()
			.is_success(),
		"the token should work before it is regenerated"
	);
	let cache_key = keys::auth_data_for_token(&hex::encode(Sha256::digest(&runner.token)));
	assert!(
		setup.get_redis_value(&cache_key).await.is_some(),
		"the token should be cached"
	);
	let lock_key = keys::runner_connection_lock(&runner.id);
	setup.set_redis_value(&lock_key, "old-connection").await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<RegenerateRunnerTokenRequest>::builder()
				.path(RegenerateRunnerTokenPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(RegenerateRunnerTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(StatusCode::ACCEPTED, response.status_code());
	let new_token = response
		.json::<ApiSuccessResponseBody<RegenerateRunnerTokenResponse>>()
		.response
		.token;

	assert_ne!(
		new_token, runner.token,
		"regenerating must rotate the token"
	);
	assert!(
		setup.get_redis_value(&cache_key).await.is_none(),
		"regenerating should drop the old token's cached entry"
	);
	assert!(
		setup.get_redis_value(&lock_key).await.is_none(),
		"regenerating should drop the runner's connection lock"
	);
	assert_eq!(
		401,
		get_workspace_info(&runner.token)
			.await
			.status_code()
			.as_u16(),
		"the old token should be rejected with 401"
	);
	assert!(
		get_workspace_info(&new_token)
			.await
			.status_code()
			.is_success(),
		"the regenerated token should work"
	);
}

/// A runner with no tunnel keeps none when its token is regenerated (its first
/// ingress token fetch creates one), and one with a tunnel gets a new one.
#[tokio::test]
async fn regenerate_runner_token_rotates_tunnel() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let regenerate = || {
		setup.make_web_dashboard_call(
			ApiRequest::<RegenerateRunnerTokenRequest>::builder()
				.path(RegenerateRunnerTokenPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(RegenerateRunnerTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
	};
	let tunnel_id = || {
		sqlx::query_scalar::<_, String>("SELECT cloudflare_tunnel_id FROM runner WHERE id = $1")
			.bind(runner.id)
			.fetch_one(setup.database())
	};

	assert_eq!(StatusCode::ACCEPTED, regenerate().await.status_code());
	assert_eq!(
		"",
		tunnel_id().await.expect("tunnel query"),
		"a runner with no tunnel should still have none"
	);

	setup
		.execute_sql(&format!(
			"UPDATE runner SET cloudflare_tunnel_id = 'old-tunnel' WHERE id = '{}'",
			runner.id
		))
		.await;

	assert_eq!(StatusCode::ACCEPTED, regenerate().await.status_code());
	let rotated = tunnel_id().await.expect("tunnel query");
	assert!(
		!rotated.is_empty() && rotated != "old-tunnel",
		"regenerating should swap the runner's tunnel, got `{rotated}`"
	);
}

/// A tunnel already gone from Cloudflare, whether it reads back as deleted or
/// isn't found at all, counts as deleted: regenerating the token still swaps
/// it, and deleting the runner still goes through.
#[tokio::test]
async fn runner_tunnel_already_gone_from_cloudflare() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	for gone_tunnel_id in ["deleted-tunnel-id", "missing-tunnel-id"] {
		let runner = setup
			.create_test_runner(&user.access_token, workspace.id)
			.await;
		let set_gone_tunnel = format!(
			"UPDATE runner SET cloudflare_tunnel_id = '{gone_tunnel_id}' WHERE id = '{}'",
			runner.id
		);

		setup.execute_sql(&set_gone_tunnel).await;
		let response = setup
			.make_web_dashboard_call(
				ApiRequest::<RegenerateRunnerTokenRequest>::builder()
					.path(RegenerateRunnerTokenPath {
						workspace_id: workspace.id,
						runner_id: runner.id,
					})
					.headers(RegenerateRunnerTokenRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
			)
			.await;
		assert_eq!(
			StatusCode::ACCEPTED,
			response.status_code(),
			"regenerating with tunnel `{gone_tunnel_id}` should succeed"
		);
		let rotated = sqlx::query_scalar::<_, String>(
			"SELECT cloudflare_tunnel_id FROM runner WHERE id = $1",
		)
		.bind(runner.id)
		.fetch_one(setup.database())
		.await
		.expect("tunnel query");
		assert!(
			!rotated.is_empty() && rotated != gone_tunnel_id,
			"regenerating should give the runner a new tunnel, got `{rotated}`"
		);

		setup.execute_sql(&set_gone_tunnel).await;
		let response = setup
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
			StatusCode::ACCEPTED,
			response.status_code(),
			"deleting a runner with tunnel `{gone_tunnel_id}` should succeed"
		);
	}
}

/// A runner can't have its token regenerated through another workspace, even
/// by someone who owns both, and its token keeps working.
#[tokio::test]
async fn regenerate_runner_token_cross_workspace() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace_a = setup.create_test_workspace(&user.access_token).await;
	let workspace_b = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace_a.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<RegenerateRunnerTokenRequest>::builder()
				.path(RegenerateRunnerTokenPath {
					workspace_id: workspace_b.id,
					runner_id: runner.id,
				})
				.headers(RegenerateRunnerTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(
		401,
		response.status_code().as_u16(),
		"a runner from another workspace should be refused (401, anti-enumeration)"
	);

	setup
		.make_api_call(
			ApiRequest::<GetWorkspaceInfoRequest>::builder()
				.path(GetWorkspaceInfoPath {
					workspace_id: workspace_a.id,
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

/// The authorizer only proves the id is some resource in the workspace, so
/// regenerating with a deployment's id must be refused and change nothing.
#[tokio::test]
async fn regenerate_runner_token_of_another_type_is_not_found() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<RegenerateRunnerTokenRequest>::builder()
				.path(RegenerateRunnerTokenPath {
					workspace_id: workspace.id,
					runner_id: deployment.id,
				})
				.headers(RegenerateRunnerTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert_eq!(StatusCode::NOT_FOUND, response.status_code());

	setup
		.make_web_dashboard_call(
			ApiRequest::<GetDeploymentInfoRequest>::builder()
				.path(GetDeploymentInfoPath {
					workspace_id: workspace.id,
					deployment_id: deployment.id,
				})
				.headers(GetDeploymentInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_status_ok();
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

/// Deleting a runner revokes its token at once, even once cached, and drops
/// its connection lock.
#[tokio::test]
async fn deleted_runner_token_is_rejected_immediately() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let get_workspace_info = || {
		setup.make_api_call(
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
	};

	assert!(
		get_workspace_info().await.status_code().is_success(),
		"the token should work before the runner is deleted"
	);
	let cache_key = keys::auth_data_for_token(&hex::encode(Sha256::digest(&runner.token)));
	assert!(
		setup.get_redis_value(&cache_key).await.is_some(),
		"the token should be cached"
	);
	let lock_key = keys::runner_connection_lock(&runner.id);
	setup.set_redis_value(&lock_key, "old-connection").await;

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

	assert!(
		setup.get_redis_value(&cache_key).await.is_none(),
		"deleting should drop the cached entry"
	);
	assert!(
		setup.get_redis_value(&lock_key).await.is_none(),
		"deleting should drop the runner's connection lock"
	);
	assert_eq!(
		401,
		get_workspace_info().await.status_code().as_u16(),
		"a deleted runner's token should be rejected with 401"
	);
}

/// A new runner has no tunnel until it first fetches its ingress token, which
/// creates one.
#[tokio::test]
async fn get_ingress_token_creates_tunnel() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let tunnel_id = || {
		sqlx::query_scalar::<_, String>("SELECT cloudflare_tunnel_id FROM runner WHERE id = $1")
			.bind(runner.id)
			.fetch_one(setup.database())
	};

	assert_eq!(
		"",
		tunnel_id().await.expect("tunnel query"),
		"creating a runner should not create a tunnel"
	);

	setup
		.make_api_call(
			ApiRequest::<GetIngressTokenForRunnerRequest>::builder()
				.path(GetIngressTokenForRunnerPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(GetIngressTokenForRunnerRequestHeaders {
					authorization: BearerToken::from_str(&runner.token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_status_ok();

	assert!(
		!tunnel_id().await.expect("tunnel query").is_empty(),
		"the first ingress token fetch should store the new tunnel"
	);
}

/// Whether the runner shows as connected.
async fn is_connected(setup: &TestSetup, runner_id: Uuid) -> bool {
	sqlx::query_scalar::<_, bool>("SELECT is_connected FROM runner WHERE id = $1")
		.bind(runner_id)
		.fetch_one(setup.database())
		.await
		.expect("is_connected query")
}

/// Connect the runner to its data stream with `token`, hand shake as a private
/// runner, and wait until the API shows it connected.
async fn connect_runner(
	setup: &TestSetup,
	workspace_id: Uuid,
	runner_id: Uuid,
	token: &str,
) -> WebSocketStream<MaybeTlsStream<TcpStream>> {
	let mut websocket = setup
		.connect_api_websocket(
			StreamRunnerDataForWorkspacePath {
				workspace_id,
				runner_id,
			},
			StreamRunnerDataForWorkspaceRequestHeaders {
				authorization: BearerToken::from_str(token).unwrap(),
				user_agent: TEST_USER_AGENT,
			},
		)
		.await
		.expect("the runner should connect");
	websocket
		.send(Message::Text(
			serde_json::to_string(&StreamRunnerDataForWorkspaceClientMsg::Handshake {
				version: semver::Version::new(0, 18, 0),
				exposure_type: RunnerExposureType::Private,
			})
			.unwrap()
			.into(),
		))
		.await
		.expect("failed to send the handshake");

	for _ in 0..50 {
		if is_connected(setup, runner_id).await {
			return websocket;
		}
		tokio::time::sleep(Duration::from_millis(100)).await;
	}
	panic!("the runner never showed as connected");
}

/// Whether the server closes `websocket` within `timeout`. Anything else it
/// sends, such as pings, is skipped.
async fn closes_within(
	websocket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
	timeout: Duration,
) -> bool {
	tokio::time::timeout(timeout, async {
		while let Some(message) = websocket.next().await {
			if matches!(message, Ok(Message::Close(_)) | Err(_)) {
				break;
			}
		}
	})
	.await
	.is_ok()
}

/// Only a runner's own token can fetch its ingress token, not a person's,
/// however much they can do with the runner.
#[tokio::test]
async fn get_ingress_token_refuses_user_tokens() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let api_token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await;
	let request = |authorization: BearerToken| {
		ApiRequest::<GetIngressTokenForRunnerRequest>::builder()
			.path(GetIngressTokenForRunnerPath {
				workspace_id: workspace.id,
				runner_id: runner.id,
			})
			.headers(GetIngressTokenForRunnerRequestHeaders {
				authorization,
				user_agent: TEST_USER_AGENT,
			})
			.build()
	};

	assert_eq!(
		StatusCode::UNAUTHORIZED,
		setup
			.make_api_call(request(BearerToken::from_str(&api_token.token).unwrap()))
			.await
			.status_code(),
		"a user API token should be refused"
	);
	assert!(
		!setup
			.make_web_dashboard_call(request(user.access_token.clone()))
			.await
			.status_code()
			.is_success(),
		"a web login should be refused"
	);
	assert_eq!(
		"",
		sqlx::query_scalar::<_, String>("SELECT cloudflare_tunnel_id FROM runner WHERE id = $1")
			.bind(runner.id)
			.fetch_one(setup.database())
			.await
			.expect("tunnel query"),
		"a refused fetch shouldn't create the runner's tunnel"
	);
}

/// After a regenerate, the runner's old token can't fetch the ingress token
/// for its new tunnel, and the new token can.
#[tokio::test]
async fn get_ingress_token_refuses_a_regenerated_token() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let get_ingress_token = |token: &str| {
		setup.make_api_call(
			ApiRequest::<GetIngressTokenForRunnerRequest>::builder()
				.path(GetIngressTokenForRunnerPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(GetIngressTokenForRunnerRequestHeaders {
					authorization: BearerToken::from_str(token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
	};

	get_ingress_token(&runner.token).await.assert_status_ok();

	let new_token = setup
		.make_web_dashboard_call(
			ApiRequest::<RegenerateRunnerTokenRequest>::builder()
				.path(RegenerateRunnerTokenPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(RegenerateRunnerTokenRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<RegenerateRunnerTokenResponse>>()
		.response
		.token;

	assert_eq!(
		StatusCode::UNAUTHORIZED,
		get_ingress_token(&runner.token).await.status_code(),
		"the old token should be refused"
	);
	get_ingress_token(&new_token).await.assert_status_ok();
}

/// A fetch that got past the authenticator on a cached token just before a
/// regenerate cleared it, then waited on the regenerate's lock on the runner,
/// is refused once the regenerate commits rather than handed the new tunnel.
#[tokio::test]
async fn get_ingress_token_reauthenticates_after_waiting_on_a_regenerate() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let get_ingress_token = || {
		setup.make_api_call(
			ApiRequest::<GetIngressTokenForRunnerRequest>::builder()
				.path(GetIngressTokenForRunnerPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(GetIngressTokenForRunnerRequestHeaders {
					authorization: BearerToken::from_str(&runner.token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
	};

	get_ingress_token().await.assert_status_ok();

	// Stands in for a regenerate, holding the runner's row the way it does
	let mut regenerate = setup.database().begin().await.expect("begin");
	let regenerate_pid = sqlx::query_scalar::<_, i32>("SELECT pg_backend_pid()")
		.fetch_one(&mut *regenerate)
		.await
		.expect("pid query");
	sqlx::query("SELECT 1 FROM runner WHERE id = $1 FOR UPDATE")
		.bind(runner.id)
		.execute(&mut *regenerate)
		.await
		.expect("failed to lock the runner");

	let (response, ()) = tokio::join!(get_ingress_token(), async {
		let mut fetch_is_waiting = false;
		for _ in 0..50 {
			fetch_is_waiting = sqlx::query_scalar::<_, bool>(concat!(
				"SELECT EXISTS (SELECT 1 FROM pg_stat_activity ",
				"WHERE $1 = ANY(pg_blocking_pids(pid)))"
			))
			.bind(regenerate_pid)
			.fetch_one(setup.database())
			.await
			.expect("blocked fetch query");
			if fetch_is_waiting {
				break;
			}
			tokio::time::sleep(Duration::from_millis(100)).await;
		}
		assert!(
			fetch_is_waiting,
			"the fetch should wait on the runner's lock"
		);

		sqlx::query(concat!(
			"UPDATE service_account SET token_hash = $2 FROM runner ",
			"WHERE runner.id = $1 AND service_account.id = runner.service_account_id"
		))
		.bind(runner.id)
		.bind(format!("regenerated-{}", runner.id))
		.execute(&mut *regenerate)
		.await
		.expect("failed to rotate the token");
		setup
			.state()
			.redis
			.del(keys::auth_data_for_token(&hex::encode(Sha256::digest(
				&runner.token,
			))))
			.await
			.expect("failed to clear the cached token");
		regenerate.commit().await.expect("commit");
	});

	assert_eq!(
		StatusCode::UNAUTHORIZED,
		response.status_code(),
		"the fetch should be refused, not handed the new tunnel's token"
	);
}

/// Two first fetches at once create one tunnel between them, not one each.
#[tokio::test]
async fn concurrent_ingress_token_fetches_create_one_tunnel() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let get_ingress_token = || {
		setup.make_api_call(
			ApiRequest::<GetIngressTokenForRunnerRequest>::builder()
				.path(GetIngressTokenForRunnerPath {
					workspace_id: workspace.id,
					runner_id: runner.id,
				})
				.headers(GetIngressTokenForRunnerRequestHeaders {
					authorization: BearerToken::from_str(&runner.token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
	};

	let (first, second) = tokio::join!(get_ingress_token(), get_ingress_token());
	first.assert_status_ok();
	second.assert_status_ok();

	assert_eq!(
		1,
		setup
			.cloudflare_requests()
			.await
			.iter()
			.filter(|request| {
				request.method.as_str() == "POST" && request.url.path().ends_with("/cfd_tunnel")
			})
			.count(),
		"only one of the fetches should create a tunnel"
	);
}

/// Every tunnel gets its own random secret rather than a shared constant.
#[tokio::test]
async fn created_tunnels_get_a_random_secret() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	for _ in 0..2 {
		let runner = setup
			.create_test_runner(&user.access_token, workspace.id)
			.await;
		setup
			.make_api_call(
				ApiRequest::<GetIngressTokenForRunnerRequest>::builder()
					.path(GetIngressTokenForRunnerPath {
						workspace_id: workspace.id,
						runner_id: runner.id,
					})
					.headers(GetIngressTokenForRunnerRequestHeaders {
						authorization: BearerToken::from_str(&runner.token).unwrap(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
			)
			.await
			.assert_status_ok();
	}

	let secrets = setup
		.cloudflare_requests()
		.await
		.iter()
		.filter(|request| {
			request.method.as_str() == "POST" && request.url.path().ends_with("/cfd_tunnel")
		})
		.map(|request| {
			serde_json::from_slice::<serde_json::Value>(&request.body).expect("tunnel body")
				["tunnel_secret"]
				.as_str()
				.expect("tunnel_secret should be a string")
				.to_owned()
		})
		.collect::<Vec<_>>();

	assert_eq!(2, secrets.len(), "each runner should get a tunnel");
	for secret in &secrets {
		assert_ne!("ZGVmYXVsdA==", secret, "the secret shouldn't be `default`");
		assert_eq!(
			32,
			BASE64_STANDARD
				.decode(secret)
				.expect("the secret should be base64")
				.len(),
			"the secret should be 32 bytes"
		);
	}
	assert_ne!(
		secrets[0], secrets[1],
		"each tunnel should get its own secret"
	);
}

/// A connected runner whose token is regenerated is dropped at its next ping,
/// which frees the lock and marks it disconnected. Handshaking as a private
/// runner gives it a tunnel, set up with its ingress config.
#[tokio::test]
async fn runner_stream_closes_when_its_token_is_regenerated() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let mut websocket = connect_runner(&setup, workspace.id, runner.id, &runner.token).await;

	assert!(
		setup.cloudflare_requests().await.iter().any(|request| {
			request.method.as_str() == "PUT" && request.url.path().ends_with("/configurations")
		}),
		"the handshake should create the tunnel with its ingress config"
	);
	assert!(
		!closes_within(&mut websocket, Duration::from_millis(2500)).await,
		"the stream should stay up while the token is the runner's"
	);

	assert_eq!(
		StatusCode::ACCEPTED,
		setup
			.make_web_dashboard_call(
				ApiRequest::<RegenerateRunnerTokenRequest>::builder()
					.path(RegenerateRunnerTokenPath {
						workspace_id: workspace.id,
						runner_id: runner.id,
					})
					.headers(RegenerateRunnerTokenRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.build(),
			)
			.await
			.status_code()
	);

	assert!(
		closes_within(&mut websocket, Duration::from_secs(5)).await,
		"the stream should close at the next ping"
	);
	assert!(
		setup
			.get_redis_value(&keys::runner_connection_lock(&runner.id))
			.await
			.is_none(),
		"closing should free the lock"
	);
	assert!(
		!is_connected(&setup, runner.id).await,
		"closing should mark the runner disconnected"
	);
}

/// When a runner moves machines, its new connection holds the lock by the
/// time the old one notices and closes, and the old one mustn't mark the
/// runner disconnected on its way out.
#[tokio::test]
async fn closing_an_old_runner_connection_keeps_a_newer_one_connected() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let lock_key = keys::runner_connection_lock(&runner.id);

	let mut websocket = connect_runner(&setup, workspace.id, runner.id, &runner.token).await;
	// What a newer connection does once it takes over
	setup.set_redis_value(&lock_key, "newer-connection").await;

	assert!(
		closes_within(&mut websocket, Duration::from_secs(5)).await,
		"the old connection should close once it has lost the lock"
	);
	assert!(
		is_connected(&setup, runner.id).await,
		"the old connection shouldn't mark the runner disconnected"
	);
	assert_eq!(
		Some("newer-connection"),
		setup.get_redis_value(&lock_key).await.as_deref(),
		"the old connection shouldn't touch the newer one's lock"
	);
}
