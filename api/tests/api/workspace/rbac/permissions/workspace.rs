use std::collections::BTreeMap;

use models::{api::workspace::*, rbac::Permission};

use super::{all, setup_permission_test};
use crate::prelude::*;

#[tokio::test]
async fn edit_workspace_permission_grants_access() {
	let setup = setup().await.expect("failed to setup test server");
	let (_admin, ws_id, user_b) =
		setup_permission_test(&setup, vec![(Permission::EditWorkspace, all())]).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<UpdateWorkspaceInfoRequest>::builder()
				.path(UpdateWorkspaceInfoPath {
					workspace_id: ws_id,
				})
				.headers(UpdateWorkspaceInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateWorkspaceInfoRequest {
					name: Some(random_name(8)),
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_success());
}

#[tokio::test]
async fn edit_workspace_denied_without_permission() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;

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
			ApiRequest::<UpdateWorkspaceInfoRequest>::builder()
				.path(UpdateWorkspaceInfoPath {
					workspace_id: workspace.id,
				})
				.headers(UpdateWorkspaceInfoRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(UpdateWorkspaceInfoRequest {
					name: Some(random_name(8)),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"user without editWorkspace should be denied"
	);
}

#[tokio::test]
async fn delete_workspace_denied_non_super_admin() {
	let setup = setup().await.expect("failed to setup test server");
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;

	let mut perms = BTreeMap::new();
	perms.insert(setup.get_permission_id(Permission::EditWorkspace), all());
	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, perms)
		.await;
	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteWorkspaceRequest>::builder()
				.path(DeleteWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(DeleteWorkspaceRequestHeaders {
					authorization: user_b.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"non-super-admin should not be able to delete workspace"
	);
}
