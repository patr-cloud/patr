use std::collections::BTreeMap;

use models::{
	api::workspace::volume::*,
	rbac::{Permission, VolumePermission},
};

use super::{all, exclude, include, setup_permission_test};
use crate::prelude::*;

#[tokio::test]
async fn volume_create_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Volume(VolumePermission::Create), all())],
	)
	.await;

	let response = setup
		.make_api_call(
			ApiRequest::<CreateVolumeRequest>::builder()
				.path(CreateVolumePath {
					workspace_id: ws_id,
				})
				.headers(CreateVolumeRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateVolumeRequest {
					name: random_name(8),
					size: 1,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn volume_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let volume = setup
		.create_test_volume(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(setup.get_permission_id(Permission::ViewRoles), all());
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<GetVolumeInfoRequest>::builder()
				.path(GetVolumeInfoPath {
					workspace_id: workspace.id,
					volume_id: volume.id,
				})
				.headers(GetVolumeInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without volume permissions should be denied"
	);
}

#[tokio::test]
async fn volume_include_grants_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let volume1 = setup
		.create_test_volume(&admin.access_token, workspace.id)
		.await;
	let volume2 = setup
		.create_test_volume(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Volume(VolumePermission::Delete)),
		include(&[volume1.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let r1 = setup
		.make_api_call(
			ApiRequest::<GetVolumeInfoRequest>::builder()
				.path(GetVolumeInfoPath {
					workspace_id: workspace.id,
					volume_id: volume1.id,
				})
				.headers(GetVolumeInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r1.status_code().is_success(),
		"volume1 should be accessible"
	);

	let r2 = setup
		.make_api_call(
			ApiRequest::<GetVolumeInfoRequest>::builder()
				.path(GetVolumeInfoPath {
					workspace_id: workspace.id,
					volume_id: volume2.id,
				})
				.headers(GetVolumeInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r2.status_code().is_client_error(),
		"volume2 should NOT be accessible"
	);
}

#[tokio::test]
async fn volume_exclude_denies_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let volume1 = setup
		.create_test_volume(&admin.access_token, workspace.id)
		.await;
	let volume2 = setup
		.create_test_volume(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Volume(VolumePermission::Delete)),
		exclude(&[volume2.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let r1 = setup
		.make_api_call(
			ApiRequest::<GetVolumeInfoRequest>::builder()
				.path(GetVolumeInfoPath {
					workspace_id: workspace.id,
					volume_id: volume1.id,
				})
				.headers(GetVolumeInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r1.status_code().is_success(),
		"volume1 should be accessible"
	);

	let r2 = setup
		.make_api_call(
			ApiRequest::<GetVolumeInfoRequest>::builder()
				.path(GetVolumeInfoPath {
					workspace_id: workspace.id,
					volume_id: volume2.id,
				})
				.headers(GetVolumeInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r2.status_code().is_client_error(),
		"volume2 should be excluded"
	);
}
