use std::collections::BTreeMap;

use models::{
	ApiSuccessResponseBody,
	api::workspace::domain::*,
	rbac::{DomainPermission, Permission},
};

use super::{all, exclude, grant, include, resources_scope, setup_permission_test};
use crate::prelude::*;

#[tokio::test]
async fn domain_add_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Domain(DomainPermission::Add), all())],
	)
	.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<AddDomainToWorkspaceRequest>::builder()
				.path(AddDomainToWorkspacePath {
					workspace_id: ws_id,
				})
				.headers(AddDomainToWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(AddDomainToWorkspaceRequest {
					domain: format!("{}.com", random_name(8)),
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn domain_delete_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let domain = setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Domain(DomainPermission::Delete)),
		include(&[domain.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grant(
			&admin.access_token,
			workspace.id,
			grant(role.id, resources_scope(&[domain.id])),
		)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteDomainInWorkspaceRequest>::builder()
				.path(DeleteDomainInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(DeleteDomainInWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_success(),
		"user with domain::delete should delete domain"
	);
}

#[tokio::test]
async fn domain_verify_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let domain = setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Domain(DomainPermission::Verify)),
		include(&[domain.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grant(
			&admin.access_token,
			workspace.id,
			grant(role.id, resources_scope(&[domain.id])),
		)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<VerifyDomainInWorkspaceRequest>::builder()
				.path(VerifyDomainInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(VerifyDomainInWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	// May fail due to Cloudflare API, but should not be a 403
	assert!(
		!response.status_code().is_client_error() || response.status_code().as_u16() != 403,
		"user with domain::verify should not get 403"
	);
}

#[tokio::test]
async fn domain_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let domain = setup
		.create_test_domain(&admin.access_token, workspace.id)
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
			ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
				.path(GetDomainInfoInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(GetDomainInfoInWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without domain permissions should be denied"
	);
}

#[tokio::test]
async fn domain_include_grants_only_listed_resource() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let domain1 = setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;
	let domain2 = setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Domain(DomainPermission::View)),
		include(&[domain1.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grant(
			&admin.access_token,
			workspace.id,
			grant(role.id, resources_scope(&[domain1.id])),
		)
		.await;

	let r1 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
				.path(GetDomainInfoInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain1.id,
				})
				.headers(GetDomainInfoInWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r1.status_code().is_success(),
		"domain1 should be accessible"
	);

	let r2 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
				.path(GetDomainInfoInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain2.id,
				})
				.headers(GetDomainInfoInWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r2.status_code().is_client_error(),
		"domain2 should NOT be accessible"
	);
}

#[tokio::test]
async fn domain_grant_omitting_a_resource_denies_it() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let domain1 = setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;
	let domain2 = setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Domain(DomainPermission::View)),
		include(&[domain1.id]),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_grant(
			&admin.access_token,
			workspace.id,
			grant(role.id, resources_scope(&[domain1.id])),
		)
		.await;

	let r1 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
				.path(GetDomainInfoInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain1.id,
				})
				.headers(GetDomainInfoInWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r1.status_code().is_success(),
		"domain1 should be accessible"
	);

	let r2 = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
				.path(GetDomainInfoInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain2.id,
				})
				.headers(GetDomainInfoInWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		r2.status_code().is_client_error(),
		"domain2 should be excluded"
	);
}

#[tokio::test]
async fn domain_view_does_not_grant_delete() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let domain = setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;

	let mut perms = BTreeMap::new();
	perms.insert(
		setup.get_permission_id(Permission::Domain(DomainPermission::View)),
		all(),
	);
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let r_view = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
				.path(GetDomainInfoInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(GetDomainInfoInWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(r_view.status_code().is_success());

	let r_delete = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteDomainInWorkspaceRequest>::builder()
				.path(DeleteDomainInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(DeleteDomainInWorkspaceRequestHeaders {
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

/// Add does not imply View: an add-only member can create a domain but cannot
/// read it back.
#[tokio::test]
async fn domain_add_does_not_grant_view() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Domain(DomainPermission::Add), all())],
	)
	.await;

	let created = setup
		.make_web_dashboard_call(
			ApiRequest::<AddDomainToWorkspaceRequest>::builder()
				.path(AddDomainToWorkspacePath {
					workspace_id: ws_id,
				})
				.headers(AddDomainToWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(AddDomainToWorkspaceRequest {
					domain: format!("{}.com", random_name(8)),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<AddDomainToWorkspaceResponse>>();

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
				.path(GetDomainInfoInWorkspacePath {
					workspace_id: ws_id,
					domain_id: created.response.id.id,
				})
				.headers(GetDomainInfoInWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"add-only member should not be able to view the domain"
	);
}

/// View does not imply Add: a view-only member cannot add a domain.
#[tokio::test]
async fn domain_view_does_not_grant_add() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Domain(DomainPermission::View), all())],
	)
	.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<AddDomainToWorkspaceRequest>::builder()
				.path(AddDomainToWorkspacePath {
					workspace_id: ws_id,
				})
				.headers(AddDomainToWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(AddDomainToWorkspaceRequest {
					domain: format!("{}.com", random_name(8)),
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"view-only member should not be able to add a domain"
	);
}

/// View does not imply Verify: a view-only member cannot verify a domain.
#[tokio::test]
async fn domain_view_does_not_grant_verify() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, ws_id, user_b) = setup_permission_test(
		&setup,
		vec![(Permission::Domain(DomainPermission::View), all())],
	)
	.await;
	let domain = setup.create_test_domain(&admin.access_token, ws_id).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<VerifyDomainInWorkspaceRequest>::builder()
				.path(VerifyDomainInWorkspacePath {
					workspace_id: ws_id,
					domain_id: domain.id,
				})
				.headers(VerifyDomainInWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"view-only member should not be able to verify a domain"
	);
}

/// A member with no domain permission gets a membership-gated list that
/// succeeds but is View-filtered to empty — not a 403.
#[tokio::test]
async fn domain_no_permission_list_returns_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let (admin, ws_id, user_b) =
		setup_permission_test(&setup, vec![(Permission::ViewRoles, all())]).await;
	let _domain = setup.create_test_domain(&admin.access_token, ws_id).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListDomainsInWorkspaceRequest>::builder()
				.path(ListDomainsInWorkspacePath {
					workspace_id: ws_id,
				})
				.headers(ListDomainsInWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListDomainsInWorkspaceResponse>>();
	assert!(
		response.response.domains.is_empty(),
		"a member without domain View should see an empty list, not a 403"
	);
}

/// A non-member cannot reach another workspace's domains at all.
#[tokio::test]
async fn domain_non_member_denied() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;
	let _domain = setup
		.create_test_domain(&admin.access_token, workspace.id)
		.await;
	let outsider = setup.create_test_user().await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListDomainsInWorkspaceRequest>::builder()
				.path(ListDomainsInWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListDomainsInWorkspaceRequestHeaders {
					authorization: outsider.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;
	assert!(
		response.status_code().is_client_error(),
		"a non-member should be denied access to the workspace's domains"
	);
}
