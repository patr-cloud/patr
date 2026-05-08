use std::collections::BTreeMap;

use models::{
	api::workspace::container_registry::*,
	rbac::{ContainerRegistryRepositoryPermission, Permission},
};

use super::{all, exclude, include, setup_permission_test};
use crate::prelude::*;

#[tokio::test]
async fn container_registry_create_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(
			Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Create),
			all(),
		)],
	)
	.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateContainerRepositoryRequest>::builder()
				.path(CreateContainerRepositoryPath {
					workspace_id: ws_id,
				})
				.headers(CreateContainerRepositoryRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateContainerRepositoryRequest {
					name: random_name(8),
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn container_registry_delete_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let repo = setup
		.create_test_container_repo(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::ContainerRegistryRepository(
			ContainerRegistryRepositoryPermission::Delete,
		)),
		include(&[repo.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteContainerRepositoryRequest>::builder()
				.path(DeleteContainerRepositoryPath {
					workspace_id: workspace.id,
					repository_id: repo.id,
				})
				.headers(DeleteContainerRepositoryRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_success(),
		"user with containerRegistryRepository::delete should delete repo"
	);
}

#[tokio::test]
async fn container_registry_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let repo = setup
		.create_test_container_repo(&admin.access_token, workspace.id)
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
		.make_web_dashboard_call(
			ApiRequest::<GetContainerRepositoryInfoRequest>::builder()
				.path(GetContainerRepositoryInfoPath {
					workspace_id: workspace.id,
					repository_id: repo.id,
				})
				.headers(GetContainerRepositoryInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without container registry permissions should be denied"
	);
}

#[tokio::test]
async fn container_registry_delete_include_grants_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let repo1 = setup
		.create_test_container_repo(&admin.access_token, workspace.id)
		.await;
	let repo2 = setup
		.create_test_container_repo(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::ContainerRegistryRepository(
			ContainerRegistryRepositoryPermission::Delete,
		)),
		include(&[repo1.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	// repo2 — should fail
	let r2 = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteContainerRepositoryRequest>::builder()
				.path(DeleteContainerRepositoryPath {
					workspace_id: workspace.id,
					repository_id: repo2.id,
				})
				.headers(DeleteContainerRepositoryRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r2.status_code().is_client_error(),
		"repo2 should NOT be deletable"
	);

	// repo1 — should succeed
	let r1 = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteContainerRepositoryRequest>::builder()
				.path(DeleteContainerRepositoryPath {
					workspace_id: workspace.id,
					repository_id: repo1.id,
				})
				.headers(DeleteContainerRepositoryRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r1.status_code().is_success(), "repo1 should be deletable");
}

#[tokio::test]
async fn container_registry_view_include_grants_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let repo1 = setup
		.create_test_container_repo(&admin.access_token, workspace.id)
		.await;
	let repo2 = setup
		.create_test_container_repo(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::ContainerRegistryRepository(
			ContainerRegistryRepositoryPermission::View,
		)),
		include(&[repo1.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let r1 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetContainerRepositoryInfoRequest>::builder()
				.path(GetContainerRepositoryInfoPath {
					workspace_id: workspace.id,
					repository_id: repo1.id,
				})
				.headers(GetContainerRepositoryInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r1.status_code().is_success(), "repo1 should be viewable");

	let r2 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetContainerRepositoryInfoRequest>::builder()
				.path(GetContainerRepositoryInfoPath {
					workspace_id: workspace.id,
					repository_id: repo2.id,
				})
				.headers(GetContainerRepositoryInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r2.status_code().is_client_error(),
		"repo2 should NOT be viewable"
	);
}

#[tokio::test]
async fn container_registry_view_exclude_denies_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let repo1 = setup
		.create_test_container_repo(&admin.access_token, workspace.id)
		.await;
	let repo2 = setup
		.create_test_container_repo(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::ContainerRegistryRepository(
			ContainerRegistryRepositoryPermission::View,
		)),
		exclude(&[repo2.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let r1 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetContainerRepositoryInfoRequest>::builder()
				.path(GetContainerRepositoryInfoPath {
					workspace_id: workspace.id,
					repository_id: repo1.id,
				})
				.headers(GetContainerRepositoryInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r1.status_code().is_success(), "repo1 should be viewable");

	let r2 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetContainerRepositoryInfoRequest>::builder()
				.path(GetContainerRepositoryInfoPath {
					workspace_id: workspace.id,
					repository_id: repo2.id,
				})
				.headers(GetContainerRepositoryInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r2.status_code().is_client_error(),
		"repo2 should be excluded"
	);
}

#[tokio::test]
async fn container_registry_view_does_not_grant_delete() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let repo = setup
		.create_test_container_repo(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::ContainerRegistryRepository(
			ContainerRegistryRepositoryPermission::View,
		)),
		all(),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let r_view = setup
		.make_web_dashboard_call(
			ApiRequest::<GetContainerRepositoryInfoRequest>::builder()
				.path(GetContainerRepositoryInfoPath {
					workspace_id: workspace.id,
					repository_id: repo.id,
				})
				.headers(GetContainerRepositoryInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r_view.status_code().is_success());

	let r_delete = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteContainerRepositoryRequest>::builder()
				.path(DeleteContainerRepositoryPath {
					workspace_id: workspace.id,
					repository_id: repo.id,
				})
				.headers(DeleteContainerRepositoryRequestHeaders {
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
