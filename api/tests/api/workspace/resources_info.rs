use std::collections::BTreeSet;

use models::{
	ApiSuccessResponseBody,
	api::workspace::*,
	rbac::ResourceType,
	utils::{BearerToken, Uuid},
};

use crate::prelude::*;

/// Resolve a batch of resource IDs in a workspace, returning the response body.
async fn resolve(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	resource_ids: impl IntoIterator<Item = Uuid>,
) -> GetResourcesInfoResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<GetResourcesInfoRequest>::builder()
				.path(GetResourcesInfoPath { workspace_id })
				.headers(GetResourcesInfoRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(GetResourcesInfoRequest {
					resource_ids: resource_ids.into_iter().collect::<BTreeSet<_>>(),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetResourcesInfoResponse>>()
		.response
}

#[tokio::test]
async fn resolves_name_and_type() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let response = resolve(&setup, &user.access_token, workspace.id, [runner.id]).await;

	assert_eq!(1, response.resources.len());
	let resolved = response.resources[0].as_ref().expect("runner not resolved");
	assert_eq!(runner.id, resolved.id);
	assert_eq!(Some(runner.name), resolved.name);
	assert_eq!(ResourceType::Runner, resolved.resource_type);
}

#[tokio::test]
async fn container_repo_resolves() {
	// Exercises a second COALESCE branch: `container_registry_repository.name` is
	// a plain TEXT column, where `runner.name` and most others are CITEXT cast to
	// TEXT. If the casts in the query were wrong, only one of these would work.
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;

	let response = resolve(&setup, &user.access_token, workspace.id, [repo.id]).await;

	assert_eq!(1, response.resources.len());
	let resolved = response.resources[0].as_ref().expect("repo not resolved");
	assert_eq!(repo.id, resolved.id);
	assert_eq!(Some(repo.name), resolved.name);
	assert_eq!(
		ResourceType::ContainerRegistryRepository,
		resolved.resource_type
	);
}

#[tokio::test]
async fn unresolved_id_returns_none() {
	// The frontend renders one row per requested ID and falls back to showing the
	// raw ID when it cannot be resolved, so an unknown ID must come back as a
	// `None` entry rather than being dropped from the response.
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let unknown = Uuid::new_v4();
	let response = resolve(&setup, &user.access_token, workspace.id, [unknown]).await;

	assert_eq!(1, response.resources.len());
	assert!(response.resources[0].is_none());
}

#[tokio::test]
async fn one_entry_per_requested_id() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let unknown = Uuid::new_v4();

	let response = resolve(
		&setup,
		&user.access_token,
		workspace.id,
		[runner.id, repo.id, unknown],
	)
	.await;

	assert_eq!(3, response.resources.len());

	let resolved_runner = response
		.resources
		.iter()
		.flatten()
		.find(|resource| resource.id == runner.id)
		.expect("runner missing from response");
	assert_eq!(ResourceType::Runner, resolved_runner.resource_type);

	let resolved_repo = response
		.resources
		.iter()
		.flatten()
		.find(|resource| resource.id == repo.id)
		.expect("repo missing from response");
	assert_eq!(
		ResourceType::ContainerRegistryRepository,
		resolved_repo.resource_type
	);

	// The unknown ID resolves to nothing, so exactly one slot comes back `None`.
	assert_eq!(1, response.resources.iter().filter(|r| r.is_none()).count());
}

#[tokio::test]
async fn other_workspace_resource_is_null() {
	// The query is scoped by `resource.workspace_id`. Without that, this endpoint
	// would leak resource names across workspace boundaries to any member.
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace_a = setup.create_test_workspace(&user.access_token).await;
	let workspace_b = setup.create_test_workspace(&user.access_token).await;

	let runner_in_b = setup
		.create_test_runner(&user.access_token, workspace_b.id)
		.await;

	let response = resolve(&setup, &user.access_token, workspace_a.id, [runner_in_b.id]).await;

	assert_eq!(1, response.resources.len());
	assert!(response.resources[0].is_none());
}

#[tokio::test]
async fn empty_request_returns_empty_response() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = resolve(&setup, &user.access_token, workspace.id, []).await;

	assert!(response.resources.is_empty());
}

#[tokio::test]
async fn duplicate_ids_collapse() {
	// `resource_ids` is a BTreeSet, so the same ID sent twice yields one entry.
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;

	let response = resolve(
		&setup,
		&user.access_token,
		workspace.id,
		[runner.id, runner.id],
	)
	.await;

	assert_eq!(1, response.resources.len());
}

#[tokio::test]
async fn non_member_cannot_resolve() {
	// The endpoint is behind WorkspaceMembershipAuthenticator.
	let setup = setup().await.expect("failed to setup test server");
	let owner = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&owner.access_token).await;
	let runner = setup
		.create_test_runner(&owner.access_token, workspace.id)
		.await;

	let outsider = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetResourcesInfoRequest>::builder()
				.path(GetResourcesInfoPath {
					workspace_id: workspace.id,
				})
				.headers(GetResourcesInfoRequestHeaders {
					authorization: outsider.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(GetResourcesInfoRequest {
					resource_ids: BTreeSet::from([runner.id]),
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}
