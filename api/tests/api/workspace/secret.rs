use std::{
	collections::{BTreeMap, BTreeSet},
	str::FromStr,
};

use models::{
	ApiSuccessResponseBody,
	api::workspace::secret::*,
	rbac::{Permission, RunnerPermission, WorkspacePermission},
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

/// A runner lists secrets during its full resync to catch rotations it
/// missed, using a token scoped to nothing but `Runner::Execute`.
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
	let token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(
				workspace.id,
				WorkspacePermission::Member {
					permissions: BTreeMap::from([(
						setup.get_permission_id(Permission::Runner(RunnerPermission::Execute)),
						BTreeSet::from([runner.id]),
					)]),
				},
			)]),
		)
		.await
		.token;

	let response = setup
		.make_api_call(
			ApiRequest::<ListSecretsForWorkspaceRequest>::builder()
				.path(ListSecretsForWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListSecretsForWorkspaceRequestHeaders {
					authorization: BearerToken::from_str(&token).unwrap(),
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
