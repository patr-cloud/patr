use std::collections::BTreeMap;

use api::models::permissions;
use models::{
	api::workspace::{rbac::user::RoleBindingGrant, secret::*},
	rbac::{Permission, SecretPermission, WorkspacePermission},
	utils::Uuid,
};

use super::helpers::*;
use crate::prelude::*;

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
				&basic_auth(&runner.id, &permissions::generate_service_account_token()),
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
	let token = runner.token.clone();

	let response = setup
		.make_openbao_call(
			http::Method::GET,
			&secret_path(&workspace.id, &secret.id),
			vec![(http::header::AUTHORIZATION, &basic_auth(&runner.id, &token))],
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

	// A service account that can view every secret but has no Runner::Execute.
	let role = setup
		.create_role_with_permissions(
			&user.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::Secret(SecretPermission::View))],
		)
		.await;
	let token = setup
		.create_test_service_account(
			&user.access_token,
			workspace.id,
			vec![RoleBindingGrant {
				role_id: role.id,
				resource_id: workspace.id,
			}],
		)
		.await
		.token;

	let response = setup
		.make_openbao_call(
			http::Method::GET,
			&secret_path(&workspace.id, &secret.id),
			vec![(http::header::AUTHORIZATION, &basic_auth(&runner.id, &token))],
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
	let token = runner.token.clone();

	// The secret exists, but in a workspace this runner has nothing to do with.
	let response = setup
		.make_openbao_call(
			http::Method::GET,
			&secret_path(&other_workspace.id, &other_secret.id),
			vec![(http::header::AUTHORIZATION, &basic_auth(&runner.id, &token))],
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"a runner must not read another workspace's secret, got {}",
		response.status_code()
	);
}

/// A runner authorized in its own workspace gets the same refusal for another
/// workspace's real secret, a made-up secret id, and an unknown runner — so it
/// can't learn whether a secret exists anywhere it can't read.
#[tokio::test]
async fn read_secret_refusals_do_not_reveal_existence() {
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
	let token = runner.token.clone();

	let mut responses = Vec::new();
	for (runner_id, secret_id) in [
		(runner.id, other_secret.id),
		(runner.id, Uuid::new_v4()),
		(Uuid::new_v4(), other_secret.id),
	] {
		let response = setup
			.make_openbao_call(
				http::Method::GET,
				&secret_path(&other_workspace.id, &secret_id),
				vec![(http::header::AUTHORIZATION, &basic_auth(&runner_id, &token))],
			)
			.await;
		responses.push((response.status_code(), response.text()));
	}

	assert_eq!(responses[0].0, StatusCode::FORBIDDEN);
	assert!(
		responses.iter().all(|response| *response == responses[0]),
		"every refusal must look the same, got {responses:?}"
	);
}

/// Secret values are read by runners, which authenticate as their own service
/// account. A user's API token is refused outright, however privileged it is.
#[tokio::test]
async fn read_secret_user_api_token_returns_401() {
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
			BTreeMap::from([(workspace.id, WorkspacePermission::SuperAdmin)]),
		)
		.await
		.token;

	let response = setup
		.make_openbao_call(
			http::Method::GET,
			&secret_path(&workspace.id, &secret.id),
			vec![(http::header::AUTHORIZATION, &basic_auth(&runner.id, &token))],
		)
		.await;

	assert_eq!(
		response.status_code(),
		StatusCode::UNAUTHORIZED,
		"a user API token must not read secret values"
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
	let token = runner.token.clone();

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

	let response = setup
		.make_openbao_call(
			http::Method::GET,
			&secret_path(&workspace.id, &secret.id),
			vec![(http::header::AUTHORIZATION, &basic_auth(&runner.id, &token))],
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
	let token = runner.token.clone();

	// Metadata row intact, value gone: the proxy forwards whatever OpenBao says.
	setup.delete_openbao_secret(workspace.id, secret.id).await;

	let response = setup
		.make_openbao_call(
			http::Method::GET,
			&secret_path(&workspace.id, &secret.id),
			vec![(http::header::AUTHORIZATION, &basic_auth(&runner.id, &token))],
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
	let token = other_runner.token.clone();

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
