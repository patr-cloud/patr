use models::{ApiSuccessResponseBody, api::workspace::*, utils::Uuid};

use crate::prelude::*;

pub mod container_registry;
pub mod deployment;
pub mod domain;
pub mod managed_url;
pub mod rbac;
pub mod runner;
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
					name: Some(new_name.clone()),
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
					name: Some(workspace_a.name.clone()),
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
					name: Some(random_name(8)),
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
