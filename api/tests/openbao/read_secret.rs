use std::collections::{BTreeMap, BTreeSet};

use models::{
	rbac::{Permission, RunnerPermission, SecretPermission, WorkspacePermission},
	utils::Uuid,
};

use super::helpers::*;
use crate::prelude::*;

/// Build an API token scoped to `Runner::Execute` on one runner, which is what
/// a runner is configured with.
async fn runner_token(
	setup: &TestSetup,
	user: &TestUser,
	workspace_id: Uuid,
	runner_id: Uuid,
) -> String {
	let permission_id = setup.get_permission_id(Permission::Runner(RunnerPermission::Execute));

	setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(
				workspace_id,
				WorkspacePermission::Member {
					permissions: BTreeMap::from([(
						permission_id,
						BTreeSet::from([runner_id]),
					)]),
				},
			)]),
		)
		.await
		.token
}

#[tokio::test]
async fn read_secret_no_auth_returns_401() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_openbao_call(
			http::Method::GET,
			&secret_path(&workspace.id, &secret.id),
			vec![],
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::UNAUTHORIZED,
		"expected 401 without Authorization header"
	);
}

#[tokio::test]
async fn read_secret_invalid_token_returns_401() {
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
		.make_openbao_call(
			http::Method::GET,
			&secret_path(&workspace.id, &secret.id),
			vec![(
				http::header::AUTHORIZATION,
				&basic_auth(&runner.id, "patrv1.not-a-real-token.nope"),
			)],
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::UNAUTHORIZED,
		"expected 401 with a bogus token"
	);
}

#[tokio::test]
async fn read_secret_returns_openbao_shaped_value() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;
	let token = runner_token(&setup, &user, workspace.id, runner.id).await;

	let response = setup
		.make_openbao_call(
			http::Method::GET,
			&secret_path(&workspace.id, &secret.id),
			vec![(
				http::header::AUTHORIZATION,
				&basic_auth(&runner.id, &token),
			)],
		)
		.await;

	assert_eq!(response.status_code(), StatusCode::OK);

	// The body is OpenBao's own KV v2 envelope, so any OpenBao-compatible
	// client can read it.
	let body = response.json::<serde_json::Value>();
	assert_eq!(
		body.pointer("/data/data/value").and_then(|v| v.as_str()),
		Some(secret.value.as_str()),
		"the proxied body should carry the value at data.data.value"
	);
}

#[tokio::test]
async fn read_secret_without_execute_is_denied() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;

	// A token with every secret permission but no Runner::Execute.
	let token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeMap::from([(
				workspace.id,
				WorkspacePermission::Member {
					permissions: BTreeMap::from([(
						setup.get_permission_id(Permission::Secret(SecretPermission::View)),
						BTreeSet::from([workspace.id]),
					)]),
				},
			)]),
		)
		.await
		.token;

	let response = setup
		.make_openbao_call(
			http::Method::GET,
			&secret_path(&workspace.id, &secret.id),
			vec![(
				http::header::AUTHORIZATION,
				&basic_auth(&runner.id, &token),
			)],
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::FORBIDDEN,
		"secret::view without runner::execute must not read a value"
	);
}

#[tokio::test]
async fn read_secret_from_another_workspace_is_denied() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let other_workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let other_secret = setup
		.create_test_secret(&user.access_token, other_workspace.id)
		.await;
	let token = runner_token(&setup, &user, workspace.id, runner.id).await;

	// The secret exists, but in a workspace this runner has nothing to do with.
	let response = setup
		.make_openbao_call(
			http::Method::GET,
			&secret_path(&other_workspace.id, &other_secret.id),
			vec![(
				http::header::AUTHORIZATION,
				&basic_auth(&runner.id, &token),
			)],
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"a runner must not read another workspace's secret, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn read_deleted_secret_returns_404() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;
	let token = runner_token(&setup, &user, workspace.id, runner.id).await;

	setup
		.execute_sql(&format!(
			"UPDATE secret SET deleted = NOW() WHERE id = '{}';",
			secret.id
		))
		.await;

	let response = setup
		.make_openbao_call(
			http::Method::GET,
			&secret_path(&workspace.id, &secret.id),
			vec![(
				http::header::AUTHORIZATION,
				&basic_auth(&runner.id, &token),
			)],
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::NOT_FOUND,
		"a deleted secret should read as not found"
	);
}

#[tokio::test]
async fn read_secret_missing_in_openbao_passes_through_404() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let runner = setup
		.create_test_runner(&user.access_token, workspace.id)
		.await;
	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;
	let token = runner_token(&setup, &user, workspace.id, runner.id).await;

	// Metadata row intact, value gone: the proxy forwards whatever OpenBao says.
	setup.delete_openbao_secret(workspace.id, secret.id).await;

	let response = setup
		.make_openbao_call(
			http::Method::GET,
			&secret_path(&workspace.id, &secret.id),
			vec![(
				http::header::AUTHORIZATION,
				&basic_auth(&runner.id, &token),
			)],
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::NOT_FOUND,
		"a value missing from OpenBao should surface as 404"
	);
}

#[tokio::test]
async fn read_secret_with_runner_from_another_workspace_is_denied() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let other_workspace = setup.create_test_workspace(&user.access_token).await;
	let other_runner = setup
		.create_test_runner(&user.access_token, other_workspace.id)
		.await;
	let secret = setup
		.create_test_secret(&user.access_token, workspace.id)
		.await;
	let token = runner_token(&setup, &user, other_workspace.id, other_runner.id).await;

	// The runner is real and the token is valid, but it belongs elsewhere.
	let response = setup
		.make_openbao_call(
			http::Method::GET,
			&secret_path(&workspace.id, &secret.id),
			vec![(
				http::header::AUTHORIZATION,
				&basic_auth(&other_runner.id, &token),
			)],
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"a runner from another workspace must be refused, got {}",
		response.status_code()
	);
}
