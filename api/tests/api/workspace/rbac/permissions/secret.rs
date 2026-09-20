use models::{
	ApiSuccessResponseBody,
	api::workspace::secret::*,
	rbac::{Permission, SecretPermission},
	utils::ListResourceQuery,
};

use super::{all, setup_permission_test};
use crate::prelude::*;

#[tokio::test]
async fn secret_create_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Secret(SecretPermission::Create), all())],
	)
	.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateSecretRequest>::builder()
				.path(CreateSecretPath {
					workspace_id: ws_id,
				})
				.headers(CreateSecretRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateSecretRequest {
					name: random_name(8).to_uppercase(),
					value: random_name(16),
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn secret_create_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Secret(SecretPermission::View), all())],
	)
	.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<CreateSecretRequest>::builder()
				.path(CreateSecretPath {
					workspace_id: ws_id,
				})
				.headers(CreateSecretRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateSecretRequest {
					name: random_name(8).to_uppercase(),
					value: random_name(16),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"secret::view must not grant create, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn secret_view_permission_grants_get_info() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Secret(SecretPermission::View), all())],
	)
	.await;
	let secret = setup.create_test_secret(&admin.access_token, ws_id).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetSecretInfoRequest>::builder()
				.path(GetSecretInfoPath {
					workspace_id: ws_id,
					secret_id: secret.id,
				})
				.headers(GetSecretInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn secret_view_does_not_grant_edit() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Secret(SecretPermission::View), all())],
	)
	.await;
	let secret = setup.create_test_secret(&admin.access_token, ws_id).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateSecretRequest>::builder()
				.path(UpdateSecretPath {
					workspace_id: ws_id,
					secret_id: secret.id,
				})
				.headers(UpdateSecretRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateSecretRequest {
					name: random_name(8).to_uppercase(),
					value: None,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"secret::view must not grant edit, got {}",
		response.status_code()
	);
}

#[tokio::test]
async fn secret_view_does_not_grant_delete() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Secret(SecretPermission::View), all())],
	)
	.await;
	let secret = setup.create_test_secret(&admin.access_token, ws_id).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteSecretRequest>::builder()
				.path(DeleteSecretPath {
					workspace_id: ws_id,
					secret_id: secret.id,
				})
				.headers(DeleteSecretRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"secret::view must not grant delete, got {}",
		response.status_code()
	);

	// The value must survive a refused delete.
	assert!(
		setup.read_openbao_secret(ws_id, secret.id).await.is_some(),
		"a denied delete must not destroy the value in OpenBao"
	);
}

#[tokio::test]
async fn secret_edit_permission_grants_update() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Secret(SecretPermission::Edit), all())],
	)
	.await;
	let secret = setup.create_test_secret(&admin.access_token, ws_id).await;
	let new_value = random_name(16);

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateSecretRequest>::builder()
				.path(UpdateSecretPath {
					workspace_id: ws_id,
					secret_id: secret.id,
				})
				.headers(UpdateSecretRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateSecretRequest {
					name: secret.name.clone(),
					value: Some(new_value.clone()),
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_success());
	assert_eq!(
		setup.read_openbao_secret(ws_id, secret.id).await,
		Some(new_value)
	);
}

#[tokio::test]
async fn secret_delete_permission_grants_delete() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Secret(SecretPermission::Delete), all())],
	)
	.await;
	let secret = setup.create_test_secret(&admin.access_token, ws_id).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteSecretRequest>::builder()
				.path(DeleteSecretPath {
					workspace_id: ws_id,
					secret_id: secret.id,
				})
				.headers(DeleteSecretRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn secret_list_needs_only_membership() {
	let setup = setup().await.expect("failed to setup test server");
	// A role with an unrelated permission: listing is gated on membership, not
	// on any secret permission.
	let (admin, ws_id, user_b) =
		setup_permission_test(&setup, vec![(Permission::ViewRoles, all())]).await;
	let secret = setup.create_test_secret(&admin.access_token, ws_id).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListSecretsForWorkspaceRequest>::builder()
				.path(ListSecretsForWorkspacePath {
					workspace_id: ws_id,
				})
				.headers(ListSecretsForWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
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
		response
			.response
			.secrets
			.iter()
			.any(|listed| listed.id == secret.id),
		"any workspace member should be able to list secrets"
	);
}

#[tokio::test]
async fn secret_non_member_denied() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let secret = setup
		.create_test_secret(&admin.access_token, workspace.id)
		.await;
	let stranger = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetSecretInfoRequest>::builder()
				.path(GetSecretInfoPath {
					workspace_id: workspace.id,
					secret_id: secret.id,
				})
				.headers(GetSecretInfoRequestHeaders {
					authorization: stranger.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"a non-member must not read another workspace's secret, got {}",
		response.status_code()
	);
}
