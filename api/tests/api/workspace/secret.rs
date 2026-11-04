use std::str::FromStr;

use models::{
	ApiSuccessResponseBody,
	api::workspace::{deployment::DeploymentStatus, secret::*},
	utils::{ListResourceQuery, Uuid},
};

use crate::prelude::*;

#[tokio::test]
async fn create_secret_stores_value_in_openbao() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;

	// Postgres holds the metadata; the value only ever lives in OpenBao.
	assert_eq!(
		setup.read_openbao_secret(workspace.id, secret.id).await,
		Some(secret.value.clone()),
		"the created value should be readable from OpenBao"
	);
}

#[tokio::test]
async fn create_secret_duplicate_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateSecretRequest>::builder()
				.path(CreateSecretPath {
					workspace_id: workspace.id,
				})
				.headers(CreateSecretRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateSecretRequest {
					name: secret.name.clone(),
					value: random_name(16),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"a duplicate secret name should be rejected, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn get_secret_info_returns_metadata_only() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetSecretInfoRequest>::builder()
				.path(GetSecretInfoPath {
					workspace_id: workspace.id,
					secret_id: secret.id,
				})
				.headers(GetSecretInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetSecretInfoResponse>>();

	assert_eq!(response.response.secret.id, secret.id);
	assert_eq!(response.response.secret.name, secret.name);
}

#[tokio::test]
async fn get_secret_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetSecretInfoRequest>::builder()
				.path(GetSecretInfoPath {
					workspace_id: workspace.id,
					secret_id: Uuid::nil(),
				})
				.headers(GetSecretInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"a secret that doesn't exist should not be found, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn list_secrets_returns_workspace_secrets() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let first = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;
	let second = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListSecretsForWorkspaceRequest>::builder()
				.path(ListSecretsForWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListSecretsForWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count: 100,
					page: 0,
					additional_query: (),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListSecretsForWorkspaceResponse>>();

	let ids = response
		.response
		.secrets
		.iter()
		.map(|secret| secret.id)
		.collect::<Vec<_>>();

	assert!(
		ids.contains(&first.id),
		"list should contain the first secret"
	);
	assert!(
		ids.contains(&second.id),
		"list should contain the second secret"
	);
}

#[tokio::test]
async fn list_secrets_excludes_other_workspaces() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let other_workspace = setup.create_test_workspace(&user.access_token).await;
	let other_secret = setup
		.create_test_secret(&user.access_token, other_workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListSecretsForWorkspaceRequest>::builder()
				.path(ListSecretsForWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListSecretsForWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count: 100,
					page: 0,
					additional_query: (),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListSecretsForWorkspaceResponse>>();

	assert!(
		!response
			.response
			.secrets
			.iter()
			.any(|secret| secret.id == other_secret.id),
		"a secret from another workspace must not be listed"
	);
}

#[tokio::test]
async fn list_secrets_filters_by_name() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let wanted = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;
	let other = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListSecretsForWorkspaceRequest>::builder()
				.path(ListSecretsForWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListSecretsForWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.query(ListResourceQuery {
					sort: None,
					search: SecretSearchParams {
						name: Some(wanted.name.to_lowercase()),
						..Default::default()
					},
					count: 100,
					page: 0,
					additional_query: (),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListSecretsForWorkspaceResponse>>();

	let ids = response
		.response
		.secrets
		.iter()
		.map(|secret| secret.id)
		.collect::<Vec<_>>();

	assert_eq!(
		ids,
		vec![wanted.id],
		"a name filter should match case-insensitively and exclude {}",
		other.name
	);
}

/// A runner lists secrets with its own token during its full resync, to catch
/// rotations it missed.
#[tokio::test]
async fn list_secrets_works_with_runner_token() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<ListSecretsForWorkspaceRequest>::builder()
				.path(ListSecretsForWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListSecretsForWorkspaceRequestHeaders {
					authorization: BearerToken::from_str(&runner.token).unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count: 100,
					page: 0,
					additional_query: (),
				})
				.build(),
		)
		.await;

	assert_eq!(response.status_code(), StatusCode::OK);
	assert!(
		response
			.json::<ApiSuccessResponseBody<ListSecretsForWorkspaceResponse>>()
			.response
			.secrets
			.iter()
			.any(|listed| listed.id == secret.id),
		"a runner token should list the workspace's secrets"
	);
}

#[tokio::test]
async fn list_secrets_page_out_of_bounds() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let mut responses = Vec::new();
	for page in [5, usize::MAX] {
		let response = setup
			.make_web_dashboard_call(
				ApiRequest::<ListSecretsForWorkspaceRequest>::builder()
					.path(ListSecretsForWorkspacePath {
						workspace_id: workspace.id,
					})
					.headers(ListSecretsForWorkspaceRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.query(ListResourceQuery {
						sort: None,
						search: Default::default(),
						count: 100,
						page,
						additional_query: (),
					})
					.build(),
			)
			.await;
		responses.push(response.status_code());
	}

	// Past the last page is PageOutOfBounds so the dashboard can recover; a page
	// so large its offset would overflow is refused before the query runs.
	assert_eq!(
		responses,
		vec![StatusCode::BAD_REQUEST, StatusCode::BAD_REQUEST],
		"out-of-range pages must be rejected, not answered with an empty list"
	);
}

/// OpenBao keeps every version of a value unless told otherwise, so a rotated
/// secret would stay readable in its history. Only the latest may survive.
#[tokio::test]
async fn rotating_a_secret_keeps_only_the_latest_version() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;

	for _ in 0..2 {
		setup
			.make_web_dashboard_call(
				ApiRequest::<UpdateSecretRequest>::builder()
					.path(UpdateSecretPath {
						workspace_id: workspace.id,
						secret_id: secret.id,
					})
					.headers(UpdateSecretRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(UpdateSecretRequest {
						name: secret.name.clone(),
						value: Some(random_name(16)),
					})
					.build(),
			)
			.await
			.assert_status(StatusCode::ACCEPTED);
	}

	assert_eq!(
		setup.read_openbao_versions(workspace.id, secret.id).await,
		vec!["3".to_string()],
		"only the latest version may be kept after rotating"
	);
}

#[tokio::test]
async fn update_secret_name_only_keeps_value() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;
	let new_name = random_name(8).to_uppercase();
	let before = setup
		.make_web_dashboard_call(
			ApiRequest::<GetSecretInfoRequest>::builder()
				.path(GetSecretInfoPath {
					workspace_id: workspace.id,
					secret_id: secret.id,
				})
				.headers(GetSecretInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetSecretInfoResponse>>()
		.response
		.secret;

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateSecretRequest>::builder()
				.path(UpdateSecretPath {
					workspace_id: workspace.id,
					secret_id: secret.id,
				})
				.headers(UpdateSecretRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateSecretRequest {
					name: new_name.clone(),
					value: None,
				})
				.build(),
		)
		.await;

	// An omitted value must leave OpenBao untouched — this is the rotation
	// path the dashboard uses when only renaming.
	assert_eq!(
		setup.read_openbao_secret(workspace.id, secret.id).await,
		Some(secret.value.clone()),
		"renaming a secret must not change its value"
	);

	let info = setup
		.make_web_dashboard_call(
			ApiRequest::<GetSecretInfoRequest>::builder()
				.path(GetSecretInfoPath {
					workspace_id: workspace.id,
					secret_id: secret.id,
				})
				.headers(GetSecretInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetSecretInfoResponse>>();

	assert_eq!(info.response.secret.name, new_name);
	// `last_updated` tracks the value, so a rename alone must not bump it —
	// otherwise every deployment using the secret restarts for nothing.
	assert_eq!(
		info.response.secret.last_updated, before.last_updated,
		"renaming a secret must not bump last_updated"
	);
}

#[tokio::test]
async fn update_secret_rotates_value() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;
	let new_value = random_name(16);
	let before = setup
		.make_web_dashboard_call(
			ApiRequest::<GetSecretInfoRequest>::builder()
				.path(GetSecretInfoPath {
					workspace_id: workspace.id,
					secret_id: secret.id,
				})
				.headers(GetSecretInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetSecretInfoResponse>>()
		.response
		.secret;

	setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateSecretRequest>::builder()
				.path(UpdateSecretPath {
					workspace_id: workspace.id,
					secret_id: secret.id,
				})
				.headers(UpdateSecretRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateSecretRequest {
					name: secret.name.clone(),
					value: Some(new_value.clone()),
				})
				.build(),
		)
		.await;

	assert_eq!(
		setup.read_openbao_secret(workspace.id, secret.id).await,
		Some(new_value),
		"a supplied value should overwrite the one in OpenBao"
	);
	let after = setup
		.make_web_dashboard_call(
			ApiRequest::<GetSecretInfoRequest>::builder()
				.path(GetSecretInfoPath {
					workspace_id: workspace.id,
					secret_id: secret.id,
				})
				.headers(GetSecretInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetSecretInfoResponse>>()
		.response
		.secret;

	assert!(
		after.last_updated > before.last_updated,
		"rotating a secret must bump last_updated"
	);
}

#[tokio::test]
async fn delete_secret_in_use_by_deployment_is_refused() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;

	// Point an env var at the secret directly: `create_test_deployment` has no
	// way to pass environment variables.
	setup
		.execute_sql(&format!(
			"INSERT INTO deployment_environment_variable(deployment_id, workspace_id, name, value, \
			 secret_id) VALUES ('{}', '{}', 'API_KEY', NULL, '{}');",
			deployment.id, workspace.id, secret.id
		))
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteSecretRequest>::builder()
				.path(DeleteSecretPath {
					workspace_id: workspace.id,
					secret_id: secret.id,
				})
				.headers(DeleteSecretRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"a secret a deployment still references must not be deletable, got {}",
		response.status_code()
	);

	// The refused delete must not have destroyed the value.
	assert_eq!(
		setup.read_openbao_secret(workspace.id, secret.id).await,
		Some(secret.value.clone()),
		"a refused delete must leave the value in OpenBao"
	);
}

#[tokio::test]
async fn delete_secret_removes_metadata_and_value() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteSecretRequest>::builder()
				.path(DeleteSecretPath {
					workspace_id: workspace.id,
					secret_id: secret.id,
				})
				.headers(DeleteSecretRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert_eq!(
		setup.read_openbao_secret(workspace.id, secret.id).await,
		None,
		"deleting a secret should destroy its value in OpenBao"
	);

	let info = setup
		.make_web_dashboard_call(
			ApiRequest::<GetSecretInfoRequest>::builder()
				.path(GetSecretInfoPath {
					workspace_id: workspace.id,
					secret_id: secret.id,
				})
				.headers(GetSecretInfoRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		info.status_code().is_client_error(),
		"a deleted secret should no longer be found, got {}",
		info.status_code()
	);
}

// ---------- create: name length bounds ----------

/// Secret names share `RESOURCE_NAME_REGEX`, whose floor is two characters
/// rather than four — plenty of real environment keys are shorter than four
/// (`ID`, `DB`, `PAT`).
#[tokio::test]
async fn create_secret_name_length_bounds() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	for (name, expect_ok) in [
		("A".to_string(), false),
		("HI".to_string(), true),
		("PAT".to_string(), true),
		("A".repeat(255), true),
		("A".repeat(256), false),
	] {
		let response = setup
			.make_web_dashboard_call(
				ApiRequest::<CreateSecretRequest>::builder()
					.path(CreateSecretPath {
						workspace_id: workspace.id,
					})
					.headers(CreateSecretRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(CreateSecretRequest {
						name: name.clone(),
						value: random_name(16),
					})
					.build(),
			)
			.await;

		if expect_ok {
			assert!(
				response.status_code().is_success(),
				"a {}-char secret name should be accepted, got {}",
				name.len(),
				response.status_code()
			);
		} else {
			assert!(
				response.status_code().is_client_error(),
				"a {}-char secret name should be rejected, got {}",
				name.len(),
				response.status_code()
			);
		}
	}
}

/// Point a deployment's env var at a secret directly: `create_test_deployment`
/// has no way to pass environment variables.
pub async fn use_secret_as(
	setup: &TestSetup,
	workspace_id: Uuid,
	deployment_id: Uuid,
	secret_id: Uuid,
	variable: &str,
) {
	setup
		.execute_sql(&format!(
			"INSERT INTO deployment_environment_variable(deployment_id, workspace_id, name, value, \
			 secret_id) VALUES ('{deployment_id}', '{workspace_id}', '{variable}', NULL, \
			 '{secret_id}');"
		))
		.await;
}

/// Send a list-secret-deployments request and return the raw response.
pub async fn list_secret_deployments(
	setup: &TestSetup,
	token: &BearerToken,
	workspace_id: Uuid,
	secret_id: Uuid,
	page: usize,
	count: usize,
) -> axum_test::TestResponse {
	setup
		.make_web_dashboard_call(
			ApiRequest::<ListSecretDeploymentsRequest>::builder()
				.path(ListSecretDeploymentsPath {
					workspace_id,
					secret_id,
				})
				.headers(ListSecretDeploymentsRequestHeaders {
					authorization: token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count,
					page,
					additional_query: (),
				})
				.build(),
		)
		.await
}

#[tokio::test]
async fn list_secret_deployments_returns_deployments_and_variables() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;
	let other_secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;

	let first = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	let second = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	let unrelated = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	use_secret_as(&setup, workspace.id, first.id, secret.id, "API_KEY").await;
	use_secret_as(&setup, workspace.id, second.id, secret.id, "PAYMENT_KEY").await;
	use_secret_as(
		&setup,
		workspace.id,
		unrelated.id,
		other_secret.id,
		"API_KEY",
	)
	.await;

	let response =
		list_secret_deployments(&setup, &user.access_token, workspace.id, secret.id, 0, 100).await;
	assert_eq!("2", response.header("x-total-count"));
	let deployments = response
		.json::<ApiSuccessResponseBody<ListSecretDeploymentsResponse>>()
		.response
		.deployments;

	let mut expected = vec![
		(first.name.to_lowercase(), first.id, "API_KEY"),
		(second.name.to_lowercase(), second.id, "PAYMENT_KEY"),
	];
	expected.sort_by_key(|(name, ..)| name.clone());

	assert_eq!(
		deployments
			.iter()
			.map(|deployment| (
				deployment.name.to_lowercase(),
				deployment.id,
				deployment.variables.clone()
			))
			.collect::<Vec<_>>(),
		expected
			.into_iter()
			.map(|(name, id, variable)| (name, id, vec![variable.to_string()]))
			.collect::<Vec<_>>(),
		"only this secret's deployments should be listed, sorted by name"
	);
	assert!(
		deployments
			.iter()
			.all(|deployment| deployment.status == DeploymentStatus::Stopped),
		"each deployment should carry its status"
	);
}

/// The primary key is `(deployment_id, name)`, so one deployment can use a
/// secret under several names. That's still one deployment.
#[tokio::test]
async fn list_secret_deployments_groups_variables_by_deployment() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;
	use_secret_as(&setup, workspace.id, deployment.id, secret.id, "B_KEY").await;
	use_secret_as(&setup, workspace.id, deployment.id, secret.id, "A_KEY").await;

	let response =
		list_secret_deployments(&setup, &user.access_token, workspace.id, secret.id, 0, 100).await;
	assert_eq!("1", response.header("x-total-count"));
	let deployments = response
		.json::<ApiSuccessResponseBody<ListSecretDeploymentsResponse>>()
		.response
		.deployments;

	assert_eq!(deployments.len(), 1);
	assert_eq!(deployments[0].id, deployment.id);
	assert_eq!(deployments[0].variables, vec!["A_KEY", "B_KEY"]);
}

#[tokio::test]
async fn list_secret_deployments_of_an_unused_secret_is_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;

	let response =
		list_secret_deployments(&setup, &user.access_token, workspace.id, secret.id, 0, 100).await;

	response.assert_status(StatusCode::OK);
	assert_eq!("0", response.header("x-total-count"));
	assert!(
		response
			.json::<ApiSuccessResponseBody<ListSecretDeploymentsResponse>>()
			.response
			.deployments
			.is_empty()
	);
}

#[tokio::test]
async fn list_secret_deployments_paginates_by_deployment() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;
	for _ in 0..3 {
		let deployment = setup
			.create_test_deployment(&user.access_token, workspace.id, runner.id)
			.await;
		// Two variables each, so counting rows instead of deployments would show.
		use_secret_as(&setup, workspace.id, deployment.id, secret.id, "A_KEY").await;
		use_secret_as(&setup, workspace.id, deployment.id, secret.id, "B_KEY").await;
	}

	let mut ids = Vec::new();
	for (page, expected_len) in [(0, 2), (1, 1)] {
		let response =
			list_secret_deployments(&setup, &user.access_token, workspace.id, secret.id, page, 2)
				.await;
		assert_eq!("3", response.header("x-total-count"));
		let deployments = response
			.json::<ApiSuccessResponseBody<ListSecretDeploymentsResponse>>()
			.response
			.deployments;
		assert_eq!(deployments.len(), expected_len, "page {page}");
		ids.extend(deployments.iter().map(|deployment| deployment.id));
	}
	ids.sort();
	ids.dedup();
	assert_eq!(ids.len(), 3, "pages should not repeat a deployment");

	list_secret_deployments(&setup, &user.access_token, workspace.id, secret.id, 5, 2)
		.await
		.assert_status(StatusCode::BAD_REQUEST);
}

/// The authenticator only proves the id is some resource in the workspace, so
/// the handler has to check that it's a secret.
#[tokio::test]
async fn list_secret_deployments_refuses_an_id_of_another_type() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let deployment = setup
		.create_test_deployment(&user.access_token, workspace.id, runner.id)
		.await;

	for id in [deployment.id, runner.id, workspace.id] {
		list_secret_deployments(&setup, &user.access_token, workspace.id, id, 0, 100)
			.await
			.assert_status(StatusCode::NOT_FOUND);
	}
}
