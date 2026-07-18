use std::collections::BTreeMap;

use models::{
	ApiSuccessResponseBody,
	api::workspace::{deployment::*, volume::*},
	utils::Uuid,
};

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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
					name: new_name.clone(),
					size: 1,
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
		.make_web_dashboard_call(
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
async fn create_volume_name_too_short() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateVolumeRequest>::builder()
				.path(CreateVolumePath {
					workspace_id: workspace.id,
				})
				.headers(CreateVolumeRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateVolumeRequest {
					name: "abc".to_string(),
					size: 1,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"volume name shorter than 4 chars should be rejected"
	);
}

#[tokio::test]
async fn create_volume_name_too_long() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateVolumeRequest>::builder()
				.path(CreateVolumePath {
					workspace_id: workspace.id,
				})
				.headers(CreateVolumeRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateVolumeRequest {
					name: "a".repeat(256),
					size: 1,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"volume name longer than 255 chars should be rejected"
	);
}

#[tokio::test]
async fn update_volume_increase_size() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let volume = setup
		.create_test_volume(&user.access_token, workspace.id)
		.await;

	setup
		.make_web_dashboard_call(
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
					name: random_name(8),
					size: 5,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(UpdateVolumeResponse));

	let response = setup
		.make_web_dashboard_call(
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

	assert_eq!(5, response.response.volume.size);
}

#[tokio::test]
async fn update_volume_decrease_size() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	// Create a volume sized 5 directly (default helper creates size 1).
	let volume_name = random_name(8);
	let volume = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateVolumeRequest>::builder()
				.path(CreateVolumePath {
					workspace_id: workspace.id,
				})
				.headers(CreateVolumeRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateVolumeRequest {
					name: volume_name,
					size: 5,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateVolumeResponse>>()
		.response;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateVolumeRequest>::builder()
				.path(UpdateVolumePath {
					workspace_id: workspace.id,
					volume_id: volume.id.id,
				})
				.headers(UpdateVolumeRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateVolumeRequest {
					name: random_name(8),
					size: 2,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"shrinking a volume should fail with CannotReduceVolumeSize"
	);
}

#[tokio::test]
async fn delete_volume_attached_to_deployment() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let volume = setup
		.create_test_volume(&user.access_token, workspace.id)
		.await;

	// Attach the volume to a deployment via create.
	let machine_types = setup
		.make_web_dashboard_call(
			ApiRequest::<ListAllDeploymentMachineTypeRequest>::builder()
				.path(ListAllDeploymentMachineTypePath {
					workspace_id: workspace.id,
				})
				.headers(ListAllDeploymentMachineTypeRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListAllDeploymentMachineTypeResponse>>();
	let machine_type_id = machine_types
		.response
		.machine_types
		.first()
		.expect("no machine types available")
		.id;

	let mut volumes = BTreeMap::new();
	volumes.insert(volume.id, "/data".to_string());

	let _ = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateDeploymentRequest>::builder()
				.path(CreateDeploymentPath {
					workspace_id: workspace.id,
				})
				.headers(CreateDeploymentRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateDeploymentRequest {
					name: random_name(8),
					registry: DeploymentRegistry::ExternalRegistry {
						registry: "docker.io".to_string(),
						image_name: "library/nginx".to_string(),
					},
					image_tag: "latest".to_string(),
					runner: runner.id,
					machine_type: machine_type_id,
					running_details: DeploymentRunningDetails {
						deploy_on_push: false,
						min_horizontal_scale: 1,
						max_horizontal_scale: 1,
						ports: BTreeMap::new(),
						environment_variables: BTreeMap::new(),
						startup_probe: None,
						liveness_probe: None,
						config_mounts: BTreeMap::new(),
						volumes,
					},
					deploy_on_create: false,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<CreateDeploymentResponse>>();

	let response = setup
		.make_web_dashboard_call(
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
		.await;

	assert!(
		response.status_code().is_client_error(),
		"deleting a volume attached to a deployment should fail with ResourceInUse"
	);
}

#[tokio::test]
async fn volume_cross_workspace() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace_a = setup.create_test_workspace(&user.access_token).await;
	let workspace_b = setup.create_test_workspace(&user.access_token).await;
	let volume = setup
		.create_test_volume(&user.access_token, workspace_a.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetVolumeInfoRequest>::builder()
				.path(GetVolumeInfoPath {
					workspace_id: workspace_b.id,
					volume_id: volume.id,
				})
				.headers(GetVolumeInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"volume in workspace A should not be accessible from workspace B"
	);
}

#[tokio::test]
async fn volume_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
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
