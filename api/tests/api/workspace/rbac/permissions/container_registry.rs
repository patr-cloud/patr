use std::collections::BTreeMap;

use models::{
	ApiSuccessResponseBody,
	api::workspace::container_registry::*,
	rbac::{ContainerRegistryRepositoryPermission, Permission},
};

use super::{all, grant, include, resources_scope, setup_permission_test};
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

		let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, vec![setup.get_permission_id(Permission::ContainerRegistryRepository(
			ContainerRegistryRepositoryPermission::Delete,
		))])
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grant(
			&admin.access_token,
			workspace.id,
			grant(role.id, resources_scope(&[repo.id])),
		)
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

		let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, vec![setup.get_permission_id(Permission::ViewRoles)])
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

		let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, vec![setup.get_permission_id(Permission::ContainerRegistryRepository(
			ContainerRegistryRepositoryPermission::Delete,
		))])
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grant(
			&admin.access_token,
			workspace.id,
			grant(role.id, resources_scope(&[repo1.id])),
		)
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

		let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, vec![setup.get_permission_id(Permission::ContainerRegistryRepository(
			ContainerRegistryRepositoryPermission::View,
		))])
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grant(
			&admin.access_token,
			workspace.id,
			grant(role.id, resources_scope(&[repo1.id])),
		)
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
async fn container_registry_view_grant_omitting_a_resource_denies_it() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let repo1 = setup
		.create_test_container_repo(&admin.access_token, workspace.id)
		.await;
	let repo2 = setup
		.create_test_container_repo(&admin.access_token, workspace.id)
		.await;

		let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, vec![setup.get_permission_id(Permission::ContainerRegistryRepository(
			ContainerRegistryRepositoryPermission::View,
		))])
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grant(
			&admin.access_token,
			workspace.id,
			grant(role.id, resources_scope(&[repo1.id])),
		)
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

/// Create does not imply View: a create-only member can create a repo but
/// cannot read it back.
#[tokio::test]
async fn container_registry_create_does_not_grant_view() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(
			Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Create),
			all(),
		)],
	)
	.await;

	let created = setup
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
		.await
		.json::<ApiSuccessResponseBody<CreateContainerRepositoryResponse>>();

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetContainerRepositoryInfoRequest>::builder()
				.path(GetContainerRepositoryInfoPath {
					workspace_id: ws_id,
					repository_id: created.response.id.id,
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
		"create-only member should not be able to view the repo"
	);
}

/// A member with no registry permission gets a membership-gated list that
/// succeeds but is View-filtered to empty — not a 403.
#[tokio::test]
async fn container_registry_no_permission_list_returns_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, ws_id, user_b) =
		setup_permission_test(&setup, vec![(Permission::ViewRoles, all())]).await;
	let _repo = setup
		.create_test_container_repo(&admin.access_token, ws_id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListContainerRepositoriesRequest>::builder()
				.path(ListContainerRepositoriesPath {
					workspace_id: ws_id,
				})
				.headers(ListContainerRepositoriesRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListContainerRepositoriesResponse>>();
	assert!(
		response.response.repositories.is_empty(),
		"a member without registry View should see an empty list, not a 403"
	);
}

/// A non-member cannot reach another workspace's registry at all.
#[tokio::test]
async fn container_registry_non_member_denied() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let _repo = setup
		.create_test_container_repo(&admin.access_token, workspace.id)
		.await;
	let outsider = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListContainerRepositoriesRequest>::builder()
				.path(ListContainerRepositoriesPath {
					workspace_id: workspace.id,
				})
				.headers(ListContainerRepositoriesRequestHeaders {
					authorization: outsider.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"a non-member should be denied access to the workspace's registry"
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

		let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, vec![setup.get_permission_id(Permission::ContainerRegistryRepository(
			ContainerRegistryRepositoryPermission::View,
		))])
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
