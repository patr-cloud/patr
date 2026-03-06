use models::{ApiSuccessResponseBody, api::workspace::volume::*, utils::Uuid};

use crate::prelude::*;

#[tokio::test]
async fn create_volume_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let volume = setup
		.create_test_volume(&user.access_token, workspace.id)
		.await;
	assert!(!volume.name.is_empty());
}

#[tokio::test]
async fn create_volume_invalid_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<CreateVolumeRequest>::builder()
				.path(CreateVolumePath {
					workspace_id: workspace.id,
				})
				.headers(CreateVolumeRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateVolumeRequest {
					name: "!!!".to_string(),
					size: 1,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for invalid volume name"
	);
}

#[tokio::test]
async fn list_volumes_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let _vol = setup
		.create_test_volume(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListVolumesInWorkspaceRequest>::builder()
				.path(ListVolumesInWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListVolumesInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListVolumesInWorkspaceResponse>>();

	assert_eq!(1, response.response.volumes.len());
}

#[tokio::test]
async fn list_volumes_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListVolumesInWorkspaceRequest>::builder()
				.path(ListVolumesInWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListVolumesInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListVolumesInWorkspaceResponse>>();

	assert!(response.response.volumes.is_empty());
}

#[tokio::test]
async fn get_volume_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let volume = setup
		.create_test_volume(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<GetVolumeInfoRequest>::builder()
				.path(GetVolumeInfoPath {
					workspace_id: workspace.id,
					volume_id: volume.id,
				})
				.headers(GetVolumeInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetVolumeInfoResponse>>();

	assert_eq!(volume.name, response.response.volume.name);
}

#[tokio::test]
async fn get_volume_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<GetVolumeInfoRequest>::builder()
				.path(GetVolumeInfoPath {
					workspace_id: workspace.id,
					volume_id: Uuid::nil(),
				})
				.headers(GetVolumeInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn update_volume_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let volume = setup
		.create_test_volume(&user.access_token, workspace.id)
		.await;
	let new_name = random_name(8);

	setup
		.make_api_call(
			ApiRequest::<UpdateVolumeRequest>::builder()
				.path(UpdateVolumePath {
					workspace_id: workspace.id,
					volume_id: volume.id,
				})
				.headers(UpdateVolumeRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateVolumeRequest {
					name: Some(new_name.clone()),
					size: None,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateVolumeResponse));
}

#[tokio::test]
async fn delete_volume_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let volume = setup
		.create_test_volume(&user.access_token, workspace.id)
		.await;

	setup
		.make_api_call(
			ApiRequest::<DeleteVolumeRequest>::builder()
				.path(DeleteVolumePath {
					workspace_id: workspace.id,
					volume_id: volume.id,
				})
				.headers(DeleteVolumeRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteVolumeResponse));

	// Verify it's gone
	let response = setup
		.make_api_call(
			ApiRequest::<GetVolumeInfoRequest>::builder()
				.path(GetVolumeInfoPath {
					workspace_id: workspace.id,
					volume_id: volume.id,
				})
				.headers(GetVolumeInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn delete_volume_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<DeleteVolumeRequest>::builder()
				.path(DeleteVolumePath {
					workspace_id: workspace.id,
					volume_id: Uuid::nil(),
				})
				.headers(DeleteVolumeRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn volume_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListVolumesInWorkspaceRequest>::builder()
				.path(ListVolumesInWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListVolumesInWorkspaceRequestHeaders {
					authorization: BearerToken::from_str("invalid-token").unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}
