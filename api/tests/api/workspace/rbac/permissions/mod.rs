use std::collections::{BTreeMap, BTreeSet};

use models::{
	rbac::{Permission, ResourcePermissionType},
	utils::Uuid,
};

use crate::prelude::*;

mod container_registry;
mod deployment;
mod domain;
mod managed_url;
mod membership;
mod rbac;
mod runner;
mod volume;
mod workspace;

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

	let role = setup
		.create_role_with_permissions(&admin.access_token, workspace.id, permissions)
		.await;

	let user_b = setup
		.add_user_to_workspace_with_role(&admin.access_token, workspace.id, role.id)
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
