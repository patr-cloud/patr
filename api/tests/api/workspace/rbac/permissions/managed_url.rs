use std::collections::BTreeMap;

use models::{
	ApiSuccessResponseBody,
	api::workspace::managed_url::*,
	rbac::{ManagedURLPermission, Permission},
};

use super::{all, exclude, include};
use crate::prelude::*;

#[tokio::test]
async fn managed_url_add_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let domain = setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;

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
		.make_web_dashboard_call(
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
	setup.mark_test_domain_verified(domain.id).await;
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
		.make_web_dashboard_call(
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
	setup.mark_test_domain_verified(domain.id).await;
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
		.make_web_dashboard_call(
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

#[tokio::test]
async fn managed_url_delete_include_grants_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let domain = setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;
	let url1 = setup
		.create_test_managed_url(&admin.access_token, workspace.id, domain.id)
		.await;
	let url2 = setup
		.create_test_managed_url(&admin.access_token, workspace.id, domain.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::ManagedURL(ManagedURLPermission::Delete)),
		include(&[url1]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	// url2 — should fail
	let r2 = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteManagedURLRequest>::builder()
				.path(DeleteManagedURLPath {
					workspace_id: workspace.id,
					managed_url_id: url2,
				})
				.headers(DeleteManagedURLRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r2.status_code().is_client_error(),
		"url2 should NOT be deletable"
	);

	// url1 — should succeed
	let r1 = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteManagedURLRequest>::builder()
				.path(DeleteManagedURLPath {
					workspace_id: workspace.id,
					managed_url_id: url1,
				})
				.headers(DeleteManagedURLRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r1.status_code().is_success(), "url1 should be deletable");
}

#[tokio::test]
async fn managed_url_delete_exclude_denies_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let domain = setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;
	let url1 = setup
		.create_test_managed_url(&admin.access_token, workspace.id, domain.id)
		.await;
	let url2 = setup
		.create_test_managed_url(&admin.access_token, workspace.id, domain.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::ManagedURL(ManagedURLPermission::Delete)),
		exclude(&[url2]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	// url2 — excluded, should fail
	let r2 = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteManagedURLRequest>::builder()
				.path(DeleteManagedURLPath {
					workspace_id: workspace.id,
					managed_url_id: url2,
				})
				.headers(DeleteManagedURLRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r2.status_code().is_client_error(),
		"url2 should be excluded"
	);

	// url1 — should succeed
	let r1 = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteManagedURLRequest>::builder()
				.path(DeleteManagedURLPath {
					workspace_id: workspace.id,
					managed_url_id: url1,
				})
				.headers(DeleteManagedURLRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r1.status_code().is_success(), "url1 should be deletable");
}

/// View does not imply Verify: a view-only member cannot verify a managed URL.
#[tokio::test]
async fn managed_url_view_does_not_grant_verify() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let domain = setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;
	let url_id = setup
		.create_test_managed_url(&admin.access_token, workspace.id, domain.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::ManagedURL(ManagedURLPermission::View)),
		all(),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<VerifyManagedURLConfigurationRequest>::builder()
				.path(VerifyManagedURLConfigurationPath {
					workspace_id: workspace.id,
					managed_url_id: url_id,
				})
				.headers(VerifyManagedURLConfigurationRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"view-only member should not be able to verify (requires managedURL::verify)"
	);
}

/// A member with no managedURL permission gets a membership-gated list that
/// succeeds but is View-filtered to empty — not a 403.
#[tokio::test]
async fn managed_url_no_permission_list_returns_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let domain = setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;
	let _url_id = setup
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
		.make_web_dashboard_call(
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
		.await
		.json::<ApiSuccessResponseBody<ListManagedURLResponse>>();
	assert!(
		response.response.urls.is_empty(),
		"a member without managedURL View should see an empty list, not a 403"
	);
}

/// A non-member cannot reach another workspace's managed URLs at all.
#[tokio::test]
async fn managed_url_non_member_denied() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let domain = setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;
	let _url_id = setup
		.create_test_managed_url(&admin.access_token, workspace.id, domain.id)
		.await;
	let outsider = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListManagedURLRequest>::builder()
				.path(ListManagedURLPath {
					workspace_id: workspace.id,
				})
				.headers(ListManagedURLRequestHeaders {
					authorization: outsider.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"a non-member should be denied access to the workspace's managed URLs"
	);
}

#[tokio::test]
async fn managed_url_view_does_not_grant_delete() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let domain = setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;
	let url_id = setup
		.create_test_managed_url(&admin.access_token, workspace.id, domain.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::ManagedURL(ManagedURLPermission::View)),
		all(),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	// View (list) should succeed.
	let r_list = setup
		.make_web_dashboard_call(
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
	assert!(r_list.status_code().is_success(), "view should grant list");

	// Delete should fail.
	let r_delete = setup
		.make_web_dashboard_call(
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
		r_delete.status_code().is_client_error(),
		"view permission should not grant delete"
	);
}
