use http::header;
use models::{
	ApiSuccessResponseBody,
	api::{ApiEndpoint, workspace::*},
	utils::Uuid,
};

use crate::prelude::*;

#[tokio::test]
async fn create_workspace_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;

	let ws = create_test_workspace(&setup, &user.access_token).await;
	assert!(!ws.name.is_empty());
}

#[tokio::test]
async fn create_workspace_duplicate_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			CreateWorkspaceRequest::METHOD,
			&CreateWorkspacePath.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.json(&CreateWorkspaceRequest {
			name: ws.name.clone(),
		})
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for duplicate workspace name"
	);
}

#[tokio::test]
async fn create_workspace_invalid_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;

	let response = setup
		.server
		.method(
			CreateWorkspaceRequest::METHOD,
			&CreateWorkspacePath.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.json(&CreateWorkspaceRequest {
			name: "!!!".to_string(),
		})
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for invalid workspace name"
	);
}

#[tokio::test]
async fn get_workspace_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			GetWorkspaceInfoRequest::METHOD,
			&GetWorkspaceInfoPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<GetWorkspaceInfoResponse>>();

	assert_eq!(ws.name, response.response.workspace.name);
	assert_eq!(ws.id, response.response.workspace.id);
}

#[tokio::test]
async fn get_workspace_info_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			GetWorkspaceInfoRequest::METHOD,
			&GetWorkspaceInfoPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error without auth token"
	);
}

#[tokio::test]
async fn get_workspace_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;

	let response = setup
		.server
		.method(
			GetWorkspaceInfoRequest::METHOD,
			&GetWorkspaceInfoPath {
				workspace_id: Uuid::nil(),
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for nonexistent workspace"
	);
}

#[tokio::test]
async fn update_workspace_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let new_name = random_name(8);

	setup
		.server
		.method(
			UpdateWorkspaceInfoRequest::METHOD,
			&UpdateWorkspaceInfoPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.json(&UpdateWorkspaceInfoRequest {
			name: Some(new_name.clone()),
		})
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateWorkspaceInfoResponse));

	// Verify
	let response = setup
		.server
		.method(
			GetWorkspaceInfoRequest::METHOD,
			&GetWorkspaceInfoPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<GetWorkspaceInfoResponse>>();

	assert_eq!(new_name, response.response.workspace.name);
}

#[tokio::test]
#[ignore = "workspace deletion needs audit_log FK redesign"]
async fn delete_workspace_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	setup
		.server
		.method(
			DeleteWorkspaceRequest::METHOD,
			&DeleteWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteWorkspaceResponse));

	// Verify it's gone
	let response = setup
		.server
		.method(
			GetWorkspaceInfoRequest::METHOD,
			&GetWorkspaceInfoPath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"deleted workspace should not be found"
	);
}

#[tokio::test]
async fn delete_workspace_not_super_admin() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &admin.access_token).await;
	let other_user = create_test_user(&setup).await;

	let response = setup
		.server
		.method(
			DeleteWorkspaceRequest::METHOD,
			&DeleteWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&other_user.access_token)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"non-super-admin should not be able to delete workspace"
	);
}

#[tokio::test]
async fn is_name_available_true() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;

	let path = format!(
		"{}?name={}",
		IsWorkspaceNameAvailablePath.to_string(),
		random_name(8)
	);
	let response = setup
		.server
		.method(IsWorkspaceNameAvailableRequest::METHOD, &path)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<IsWorkspaceNameAvailableResponse>>();

	assert!(response.response.available, "unused name should be available");
}

#[tokio::test]
async fn is_name_available_false() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let path = format!(
		"{}?name={}",
		IsWorkspaceNameAvailablePath.to_string(),
		ws.name
	);
	let response = setup
		.server
		.method(IsWorkspaceNameAvailableRequest::METHOD, &path)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<IsWorkspaceNameAvailableResponse>>();

	assert!(
		!response.response.available,
		"taken name should not be available"
	);
}
