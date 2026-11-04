use models::{
	ApiSuccessResponseBody,
	api::workspace::secret::*,
	rbac::{DeploymentPermission, Permission, SecretPermission},
	utils::ListResourceQuery,
};

use super::{all, grants, setup_permission_test};
use crate::{
	api::workspace::secret::{list_secret_deployments, use_secret_as},
	prelude::*,
};

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

/// A member without `secret::view` gets an empty list, not every secret.
#[tokio::test]
async fn secret_list_hides_secrets_without_view() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, ws_id, user_b) =
		setup_permission_test(&setup, vec![(Permission::ViewRoles, all())]).await;
	setup.create_test_secret(&admin.access_token, ws_id).await;

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
		.await;

	assert_eq!("0", response.header("x-total-count"));
	let body = response.json::<ApiSuccessResponseBody<ListSecretsForWorkspaceResponse>>();
	assert!(body.response.secrets.is_empty());
}

/// The total count only covers the secrets the member can view.
#[tokio::test]
async fn secret_view_include_list_counts_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let secret1 = setup
		.create_test_secret(&admin.access_token, workspace.id)
		.await;
	setup
		.create_test_secret(&admin.access_token, workspace.id)
		.await;

	let role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::Secret(SecretPermission::View))],
		)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grants(
			&admin.access_token,
			workspace.id,
			grants(role.id, &[secret1.id]),
		)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListSecretsForWorkspaceRequest>::builder()
				.path(ListSecretsForWorkspacePath {
					workspace_id: workspace.id,
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
		.await;

	assert_eq!("1", response.header("x-total-count"));
	let body = response.json::<ApiSuccessResponseBody<ListSecretsForWorkspaceResponse>>();
	assert_eq!(1, body.response.secrets.len());
	assert_eq!(secret1.id, body.response.secrets[0].id);
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

/// `secret::view` doesn't imply `deployment::view`, so the list must not reveal
/// the names or variables of deployments the caller can't view.
#[tokio::test]
async fn secret_deployments_only_lists_deployments_the_caller_can_view() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let runner = setup
		.create_test_runner(&admin.access_token, workspace.id)
		.await;
	let secret = setup
		.create_test_secret(&admin.access_token, workspace.id)
		.await;
	let visible = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;
	let hidden = setup
		.create_test_deployment(&admin.access_token, workspace.id, runner.id)
		.await;
	use_secret_as(&setup, workspace.id, visible.id, secret.id, "API_KEY").await;
	use_secret_as(&setup, workspace.id, hidden.id, secret.id, "API_KEY").await;

	let secret_role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::Secret(SecretPermission::View))],
		)
		.await;
	let deployment_role = setup
		.create_role_with_permissions(
			&admin.access_token,
			workspace.id,
			vec![setup.get_permission_id(Permission::Deployment(DeploymentPermission::View))],
		)
		.await;
	let mut member_grants = grants(secret_role.id, &[workspace.id]);
	member_grants.extend(grants(deployment_role.id, &[visible.id]));
	let member = setup
		.add_user_to_workspace_with_grants(&admin.access_token, workspace.id, member_grants)
		.await;

	let response = list_secret_deployments(
		&setup,
		&member.access_token,
		workspace.id,
		secret.id,
		0,
		100,
	)
	.await;
	assert_eq!("1", response.header("x-total-count"));
	let deployments = response
		.json::<ApiSuccessResponseBody<ListSecretDeploymentsResponse>>()
		.response
		.deployments;

	assert_eq!(
		deployments
			.iter()
			.map(|deployment| deployment.id)
			.collect::<Vec<_>>(),
		vec![visible.id]
	);
}

#[tokio::test]
async fn secret_deployments_empty_without_deployment_view() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Secret(SecretPermission::View), all())],
	)
	.await;
	let runner = setup.create_test_runner(&admin.access_token, ws_id).await;
	let secret = setup.create_test_secret(&admin.access_token, ws_id).await;
	let deployment = setup
		.create_test_deployment(&admin.access_token, ws_id, runner.id)
		.await;
	use_secret_as(&setup, ws_id, deployment.id, secret.id, "API_KEY").await;

	let response =
		list_secret_deployments(&setup, &user_b.access_token, ws_id, secret.id, 0, 100).await;

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
async fn secret_deployments_denied_without_secret_view() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Deployment(DeploymentPermission::View), all())],
	)
	.await;
	let secret = setup.create_test_secret(&admin.access_token, ws_id).await;

	let response =
		list_secret_deployments(&setup, &user_b.access_token, ws_id, secret.id, 0, 100).await;

	assert!(
		response.status_code().is_client_error(),
		"deployment::view must not grant reading a secret's deployments, got {}",
		response.status_code()
	);
}
