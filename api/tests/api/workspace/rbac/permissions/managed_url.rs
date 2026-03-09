use std::collections::BTreeMap;

use models::{
	api::workspace::managed_url::*,
	rbac::{ManagedURLPermission, Permission},
};

use super::{all, include};
use crate::prelude::*;

#[tokio::test]
async fn managed_url_add_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let domain = setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::ManagedURL(ManagedURLPermission::Add)),
		all(),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<CreateManagedURLRequest>::builder()
				.path(CreateManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(CreateManagedURLRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(CreateManagedURLRequest {
					sub_domain: random_name(6),
					domain_id: domain.id,
					path: "/".to_string(),
					url_type: ManagedUrlType::Redirect {
						url: "https://example.com".to_string(),
						permanent_redirect: false,
						http_only: false,
					},
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_success(),
		"user with managedUrl::add should create managed URL"
	);
}

#[tokio::test]
async fn managed_url_delete_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let domain = setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;
	let url_id = setup
		.create_test_managed_url(&admin.access_token, workspace.id, domain.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::ManagedURL(ManagedURLPermission::Delete)),
		include(&[url_id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_api_call(
			ApiRequest::<DeleteManagedURLRequest>::builder()
				.path(DeleteManagedURLPath {
					workspace_id: workspace.id,
					managed_url_id: url_id,
				})
				.headers(DeleteManagedURLRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_success(),
		"user with managedUrl::delete should delete managed URL"
	);
}

#[tokio::test]
async fn managed_url_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let domain = setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;
	let url_id = setup
		.create_test_managed_url(&admin.access_token, workspace.id, domain.id)
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
		.make_api_call(
			ApiRequest::<UpdateManagedURLRequest>::builder()
				.path(UpdateManagedURLPath {
					workspace_id: workspace.id,
					managed_url_id: url_id,
				})
				.headers(UpdateManagedURLRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateManagedURLRequest {
					path: Some("/new".to_string()),
					url_type: None,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without managedUrl permissions should be denied"
	);
}
