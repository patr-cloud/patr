use http::header;
use models::{
	ApiSuccessResponseBody,
	api::{
		ApiEndpoint,
		workspace::volume::*,
	},
	utils::Uuid,
};

use crate::prelude::*;

#[tokio::test]
async fn create_volume_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let volume = create_test_volume(&setup, &user.access_token, ws.id).await;
	assert!(!volume.name.is_empty());
}

#[tokio::test]
async fn create_volume_invalid_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			CreateVolumeRequest::METHOD,
			&CreateVolumePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.json(&CreateVolumeRequest {
			name: "!!!".to_string(),
			size: 1,
		})
		.await;

	assert!(
		response.status_code().is_client_error(),
		"expected client error for invalid volume name"
	);
}

#[tokio::test]
async fn list_volumes_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let _vol = create_test_volume(&setup, &user.access_token, ws.id).await;

	let response = setup
		.server
		.method(
			ListVolumesInWorkspaceRequest::METHOD,
			&ListVolumesInWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListVolumesInWorkspaceResponse>>();

	assert_eq!(1, response.response.volumes.len());
}

#[tokio::test]
async fn list_volumes_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			ListVolumesInWorkspaceRequest::METHOD,
			&ListVolumesInWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<ListVolumesInWorkspaceResponse>>();

	assert!(response.response.volumes.is_empty());
}

#[tokio::test]
async fn get_volume_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let vol = create_test_volume(&setup, &user.access_token, ws.id).await;

	let response = setup
		.server
		.method(
			GetVolumeInfoRequest::METHOD,
			&GetVolumeInfoPath {
				workspace_id: ws.id,
				volume_id: vol.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.json::<ApiSuccessResponseBody<GetVolumeInfoResponse>>();

	assert_eq!(vol.name, response.response.volume.name);
}

#[tokio::test]
async fn get_volume_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			GetVolumeInfoRequest::METHOD,
			&GetVolumeInfoPath {
				workspace_id: ws.id,
				volume_id: Uuid::nil(),
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn update_volume_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let vol = create_test_volume(&setup, &user.access_token, ws.id).await;
	let new_name = random_name(8);

	setup
		.server
		.method(
			UpdateVolumeRequest::METHOD,
			&UpdateVolumePath {
				workspace_id: ws.id,
				volume_id: vol.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.json(&UpdateVolumeRequest {
			name: Some(new_name.clone()),
			size: None,
		})
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateVolumeResponse));
}

#[tokio::test]
async fn delete_volume_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;
	let vol = create_test_volume(&setup, &user.access_token, ws.id).await;

	setup
		.server
		.method(
			DeleteVolumeRequest::METHOD,
			&DeleteVolumePath {
				workspace_id: ws.id,
				volume_id: vol.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await
		.assert_json(&ApiSuccessResponseBody::new(DeleteVolumeResponse));

	// Verify it's gone
	let response = setup
		.server
		.method(
			GetVolumeInfoRequest::METHOD,
			&GetVolumeInfoPath {
				workspace_id: ws.id,
				volume_id: vol.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn delete_volume_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			DeleteVolumeRequest::METHOD,
			&DeleteVolumePath {
				workspace_id: ws.id,
				volume_id: Uuid::nil(),
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.authorization_bearer(&user.access_token)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn volume_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = create_test_user(&setup).await;
	let ws = create_test_workspace(&setup, &user.access_token).await;

	let response = setup
		.server
		.method(
			ListVolumesInWorkspaceRequest::METHOD,
			&ListVolumesInWorkspacePath {
				workspace_id: ws.id,
			}
			.to_string(),
		)
		.add_header(header::USER_AGENT, "cargo-test")
		.await;

	assert!(response.status_code().is_client_error());
}
