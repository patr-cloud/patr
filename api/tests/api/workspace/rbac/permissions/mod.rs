use models::{
	api::workspace::rbac::user::RoleGrant,
	rbac::{Permission, PermissionScope},
	utils::Uuid,
};

use crate::prelude::*;

pub mod container_registry;
pub mod deployment;
pub mod domain;
pub mod managed_url;
pub mod membership;
pub mod rbac;
pub mod runner;
pub mod volume;
pub mod workspace;

/// Create admin, workspace, and user B with a role that has specific
/// permissions. Returns (admin, workspace_id, user_b).
async fn setup_permission_test(
	setup: &TestSetup,
	perm_entries: Vec<(Permission, PermissionScope)>,
) -> (TestUser, Uuid, TestUser) {
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;

	let mut permissions = Vec::new();
	let mut scope = None;
	for (perm, perm_scope) in perm_entries {
		permissions.push(setup.get_permission_id(perm));
		// A binding applies the whole role at one scope; entries share it.
		assert!(
			scope.is_none() || scope.as_ref() == Some(&perm_scope),
			"all entries must share one scope"
		);
		scope = Some(perm_scope);
	}
	let scope = scope.expect("at least one permission");

	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, permissions)
		.await;

	let user_b = setup
		.add_user_to_workspace_with_grant(
			&admin.access_token,
			workspace.id,
			RoleGrant {
				role_id: role.id,
				scope,
			},
		)
		.await;

	(admin, workspace.id, user_b)
}

fn include(ids: &[Uuid]) -> PermissionScope {
	PermissionScope::Resources(ids.iter().copied().collect())
}

fn all() -> PermissionScope {
	PermissionScope::Workspace
}

/// A grant scope covering exactly these resources.
fn resources_scope(ids: &[Uuid]) -> PermissionScope {
	PermissionScope::Resources(ids.iter().copied().collect())
}

/// A role grant at a specific scope.
fn grant(role_id: Uuid, scope: PermissionScope) -> RoleGrant {
	RoleGrant { role_id, scope }
}
