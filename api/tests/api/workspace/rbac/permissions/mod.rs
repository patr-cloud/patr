use std::collections::{BTreeMap, BTreeSet};

use models::{
	api::workspace::rbac::user::RoleGrant,
	rbac::{Permission, PermissionScope, ResourcePermissionType},
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
	perm_entries: Vec<(Permission, ResourcePermissionType)>,
) -> (TestUser, Uuid, TestUser) {
	let admin = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&admin.access_token).await;

	let mut permissions = BTreeMap::new();
	for (perm, perm_type) in perm_entries {
		let perm_id = setup.get_permission_id(perm);
		permissions.insert(perm_id, perm_type);
	}

	// The grant's scope is derived from the (uniform) permission entries:
	// scope moved from the role to the assignment at the role-binding
	// cutover. Non-empty excludes have no additive equivalent.
	let scope = match permissions.values().next().expect("at least one permission") {
		ResourcePermissionType::Include(resources) => {
			PermissionScope::Resources(resources.clone())
		}
		ResourcePermissionType::Exclude(resources) if resources.is_empty() => {
			PermissionScope::Workspace
		}
		ResourcePermissionType::Exclude(_) => {
			panic!("non-empty excludes are not representable as grants")
		}
	};

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

fn include(ids: &[Uuid]) -> ResourcePermissionType {
	ResourcePermissionType::Include(ids.iter().copied().collect())
}

fn exclude(ids: &[Uuid]) -> ResourcePermissionType {
	ResourcePermissionType::Exclude(ids.iter().copied().collect())
}

fn all() -> ResourcePermissionType {
	ResourcePermissionType::Exclude(BTreeSet::new())
}

/// A grant scope covering exactly these resources.
fn resources_scope(ids: &[Uuid]) -> PermissionScope {
	PermissionScope::Resources(ids.iter().copied().collect())
}

/// A role grant at a specific scope.
fn grant(role_id: Uuid, scope: PermissionScope) -> RoleGrant {
	RoleGrant { role_id, scope }
}
