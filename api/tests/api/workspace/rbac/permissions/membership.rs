use std::collections::BTreeMap;

use models::{
	api::workspace::{
		container_registry::*,
		deployment::*,
		domain::*,
		managed_url::*,
		runner::*,
		volume::*,
	},
	rbac::Permission,
};

use super::all;
use crate::prelude::*;

#[tokio::test]
async fn list_deployments_denied_non_member() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let non_member = setup.create_test_user().await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListDeploymentRequest>::builder()
				.path(ListDeploymentPath {
					workspace_id: workspace.id,
				})
				.headers(ListDeploymentRequestHeaders {
					authorization: non_member.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"non-member should not be able to list deployments"
	);
}

#[tokio::test]
async fn list_volumes_denied_non_member() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let non_member = setup.create_test_user().await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListVolumesInWorkspaceRequest>::builder()
				.path(ListVolumesInWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListVolumesInWorkspaceRequestHeaders {
					authorization: non_member.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn list_runners_denied_non_member() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let non_member = setup.create_test_user().await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
				.path(ListRunnersForWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListRunnersForWorkspaceRequestHeaders {
					authorization: non_member.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn list_domains_denied_non_member() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let non_member = setup.create_test_user().await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListDomainsInWorkspaceRequest>::builder()
				.path(ListDomainsInWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListDomainsInWorkspaceRequestHeaders {
					authorization: non_member.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn list_repositories_denied_non_member() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let non_member = setup.create_test_user().await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListContainerRepositoriesRequest>::builder()
				.path(ListContainerRepositoriesPath {
					workspace_id: workspace.id,
				})
				.headers(ListContainerRepositoriesRequestHeaders {
					authorization: non_member.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn list_managed_urls_denied_non_member() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let non_member = setup.create_test_user().await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListManagedURLRequest>::builder()
				.path(ListManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(ListManagedURLRequestHeaders {
					authorization: non_member.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn list_endpoints_allowed_for_any_member() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;

	let mut perms = BTreeMap::new();
	perms.insert(setup.get_permission_id(Permission::ViewRoles), all());
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let list_deployments = setup
		.make_api_call(
			ApiRequest::<ListDeploymentRequest>::builder()
				.path(ListDeploymentPath {
					workspace_id: workspace.id,
				})
				.headers(ListDeploymentRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		list_deployments.status_code().is_success(),
		"member should list deployments"
	);

	let list_runners = setup
		.make_api_call(
			ApiRequest::<ListRunnersForWorkspaceRequest>::builder()
				.path(ListRunnersForWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListRunnersForWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		list_runners.status_code().is_success(),
		"member should list runners"
	);

	let list_volumes = setup
		.make_api_call(
			ApiRequest::<ListVolumesInWorkspaceRequest>::builder()
				.path(ListVolumesInWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListVolumesInWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		list_volumes.status_code().is_success(),
		"member should list volumes"
	);

	let list_domains = setup
		.make_api_call(
			ApiRequest::<ListDomainsInWorkspaceRequest>::builder()
				.path(ListDomainsInWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListDomainsInWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		list_domains.status_code().is_success(),
		"member should list domains"
	);

	let list_repos = setup
		.make_api_call(
			ApiRequest::<ListContainerRepositoriesRequest>::builder()
				.path(ListContainerRepositoriesPath {
					workspace_id: workspace.id,
				})
				.headers(ListContainerRepositoriesRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		list_repos.status_code().is_success(),
		"member should list repositories"
	);

	let list_urls = setup
		.make_api_call(
			ApiRequest::<ListManagedURLRequest>::builder()
				.path(ListManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(ListManagedURLRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		list_urls.status_code().is_success(),
		"member should list managed URLs"
	);
}
