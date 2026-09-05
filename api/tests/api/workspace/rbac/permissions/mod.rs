use models::{api::workspace::rbac::user::RoleBindingGrant, rbac::Permission, utils::Uuid};

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
	perm_entries: Vec<(Permission, Vec<Uuid>)>,
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
	// An empty scope list means the whole workspace: the grant sits at the root.
	let resource_ids = if scope.is_empty() {
		vec![workspace.id]
	} else {
		scope
	};

	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, permissions)
		.await;

	let user_b = setup
		.add_user_to_workspace_with_grants(
			&admin.access_token,
			workspace.id,
			grants(role.id, &resource_ids),
		)
		.await;

	(admin, workspace.id, user_b)
}

/// The whole workspace — an empty list, resolved to the workspace root.
fn all() -> Vec<Uuid> {
	Vec::new()
}

/// One grant of `role_id` per resource it applies at.
fn grants(role_id: Uuid, resource_ids: &[Uuid]) -> Vec<RoleBindingGrant> {
	resource_ids
		.iter()
		.map(|resource_id| RoleBindingGrant {
			role_id,
			resource_id: *resource_id,
		})
		.collect::<Vec<_>>()
}
