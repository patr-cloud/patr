use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{prelude::*, rbac::ResourcePermissionType};

/// Represents the kind of permission that is granted on a workspace.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, ts_rs::TS)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkspacePermission {
	/// The user is the super admin of the workspace.
	SuperAdmin,
	/// The user is a member of the workspace.
	Member {
		/// List of Permission IDs and the type of permission that is granted.
		#[serde(flatten)]
		permissions: BTreeMap<Uuid, ResourcePermissionType>,
	},
}

impl WorkspacePermission {
	/// Returns true if the user is a super admin of the workspace.
	#[must_use]
	pub fn is_super_admin(&self) -> bool {
		matches!(self, WorkspacePermission::SuperAdmin)
	}

	/// Returns true if the user is a member of the workspace.
	#[must_use]
	pub fn is_member(&self) -> bool {
		matches!(self, WorkspacePermission::Member { .. })
	}

	/// Returns true if the current [`WorkspacePermission`] instance has more or
	/// equal permissions than the other [`WorkspacePermission`] instance.
	#[must_use]
	pub fn is_superset_of(&self, other: &WorkspacePermission) -> bool {
		match (self, other) {
			// If you're a super admin, you have all permissions. So go ahead, regardless of what
			// you're requesting, you're allowed.
			(Self::SuperAdmin, _) => true,
			// If you're a member, and you're asking for super admin permissions,
			// that's disallowed.
			(Self::Member { .. }, Self::SuperAdmin) => false,
			// If you're a member, you are requesting member permissions, then we need to check
			// deeper.
			(
				Self::Member {
					permissions: self_permissions,
				},
				Self::Member {
					permissions: other_permissions,
				},
			) => other_permissions
				.iter()
				.all(|(permission_id, other_resources)| {
					let Some(self_resources) = self_permissions.get(permission_id) else {
						return false;
					};
					match (self_resources, other_resources) {
						(
							ResourcePermissionType::Include(self_resources),
							ResourcePermissionType::Include(other_resources),
						) => {
							// If you have a set of resources that you can access, and you are
							// requesting permissions for another set of resources, this is only
							// allowed if the set of resources you have access to is a superset of
							// the set of resources you are requesting.
							self_resources.is_superset(other_resources)
						}
						(
							ResourcePermissionType::Include(_),
							ResourcePermissionType::Exclude(_),
						) => {
							// If the current permission is to include a set of resources, and
							// the other permission is to exclude a set of resources, then the
							// current permission is not a subset of the other permission.
							//
							// Why? Simple example:
							// If the list of resources are [1, 2, 3, 4, 5], and the include
							// permission has a list of resources [1, 2, 3], and the exclude
							// permission has a list of resources [4], then the include permission
							// is not a subset of the exclude permission. In this case, the include
							// permission has access to resources 1, 2, 3, but the exclude
							// permission has access to resources 1, 2, 3, 5.
							//
							// The only way that the include permission would be a subset of the
							// exclude permission is if the exclude permission had a list of all
							// resources that are an exact inverse of the include permission. But
							// that also might not always work. Even if the exclude permission has a
							// list of all resources that are an exact inverse of the include
							// permission, when the user creates a new resource, the new resource
							// would be accessible by the exclude permission, but not the include
							// permission.
							//
							// So yeah, we're straight up rejecting this

							false
						}
						(
							ResourcePermissionType::Exclude(self_resources),
							ResourcePermissionType::Include(other_resources),
						) => {
							// Okay see, the user has an exclude permission, and the other
							// permission is to include a set of resources. This is a bit
							// tricky.
							//
							// If the user has an exclude permission, then the user is
							// allowed to access all resources except the ones that are in
							// the exclude list. So if the other permission is to include a
							// set of resources, then any resource is allowed, as long as it
							// is not in the exclude list.
							self_resources.is_disjoint(other_resources)
						}
						(
							ResourcePermissionType::Exclude(self_resources),
							ResourcePermissionType::Exclude(other_resources),
						) => {
							// This is tough to explain, but I'm gonna try.
							// Your current permissions are on all resources except a few. The other
							// permissions are also on all resources except a few. If the resources
							// that other permissions are excluding is bigger than the current one,
							// then that's cool. Cuz as long as others aren't accessing the
							// resources in the current list, they are free to exclude other
							// resources as well.
							self_resources.is_subset(other_resources)
						}
					}
				}),
		}
	}

	/// Returns the intersection of this [`WorkspacePermission`] with `other`.
	/// The result is the permission set that is allowed by *both* sides — used
	/// at API-token auth time to clamp a token's declared permissions by its
	/// owner's current role-derived permissions, so revoking a role
	/// automatically narrows any tokens that the role had been granted to.
	///
	/// Returns [`None`] when the intersection is empty (i.e. the resulting
	/// [`WorkspacePermission::Member`] has no permission entries), so the
	/// caller can drop the workspace entry entirely.
	#[must_use]
	pub fn intersect_with(&self, other: &WorkspacePermission) -> Option<WorkspacePermission> {
		match (self, other) {
			// SuperAdmin ∩ X = X. Drop if the other side is an empty Member
			// (caller treats that as "no perms in this workspace").
			(Self::SuperAdmin, Self::SuperAdmin) => Some(Self::SuperAdmin),
			(Self::SuperAdmin, other) | (other, Self::SuperAdmin) => match other {
				Self::Member { permissions } if permissions.is_empty() => None,
				_ => Some(other.clone()),
			},
			(
				Self::Member {
					permissions: self_permissions,
				},
				Self::Member {
					permissions: other_permissions,
				},
			) => {
				let mut intersected = BTreeMap::new();
				for (permission_id, self_resources) in self_permissions {
					let Some(other_resources) = other_permissions.get(permission_id) else {
						continue;
					};
					let combined = match (self_resources, other_resources) {
						(
							ResourcePermissionType::Include(self_res),
							ResourcePermissionType::Include(other_res),
						) => {
							// Both sides include explicit resource lists — the resources
							// allowed by both are the intersection of the two sets.
							let inter = self_res
								.intersection(other_res)
								.copied()
								.collect::<BTreeSet<_>>();
							if inter.is_empty() {
								continue;
							}
							ResourcePermissionType::Include(inter)
						}
						(
							ResourcePermissionType::Exclude(self_res),
							ResourcePermissionType::Exclude(other_res),
						) => {
							// Both sides allow all-except — the combined exclude list
							// blocks any resource that *either* side blocks.
							let union = self_res.union(other_res).copied().collect::<BTreeSet<_>>();
							ResourcePermissionType::Exclude(union)
						}
						(
							ResourcePermissionType::Include(inc),
							ResourcePermissionType::Exclude(exc),
						) |
						(
							ResourcePermissionType::Exclude(exc),
							ResourcePermissionType::Include(inc),
						) => {
							// One side names the only allowed resources; the other names
							// resources to block. The result is the included set minus
							// anything the exclude side blocks.
							let diff = inc.difference(exc).copied().collect::<BTreeSet<_>>();
							if diff.is_empty() {
								continue;
							}
							ResourcePermissionType::Include(diff)
						}
					};
					intersected.insert(*permission_id, combined);
				}
				if intersected.is_empty() {
					None
				} else {
					Some(Self::Member {
						permissions: intersected,
					})
				}
			}
		}
	}

	/// Returns true if the user has the specified permission on the given
	/// resource.
	#[must_use]
	pub fn has_permission_on_resource(&self, resource_id: Uuid, permission_id: Uuid) -> bool {
		match self {
			Self::SuperAdmin => true,
			Self::Member { permissions } => {
				permissions
					.get(&permission_id)
					.is_some_and(|resource_permissions| {
						resource_permissions.has_resource(&resource_id)
					})
			}
		}
	}
}

/// Intersects two workspace permission maps. Only workspaces present in both
/// inputs are kept, and each workspace's [`WorkspacePermission`] is intersected
/// via [`WorkspacePermission::intersect_with`]. Workspaces whose intersection
/// is empty are dropped from the result.
#[must_use]
pub fn intersect_workspace_permissions(
	left: &BTreeMap<Uuid, WorkspacePermission>,
	right: &BTreeMap<Uuid, WorkspacePermission>,
) -> BTreeMap<Uuid, WorkspacePermission> {
	left.iter()
		.filter_map(|(workspace_id, left_permission)| {
			let right_permission = right.get(workspace_id)?;
			let intersected = left_permission.intersect_with(right_permission)?;
			Some((*workspace_id, intersected))
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn create_member_permission(
		permissions: BTreeMap<Uuid, ResourcePermissionType>,
	) -> WorkspacePermission {
		WorkspacePermission::Member { permissions }
	}

	#[test]
	fn test_is_super_admin() {
		assert!(WorkspacePermission::SuperAdmin.is_super_admin());
		assert!(!create_member_permission(BTreeMap::new()).is_super_admin());
	}

	#[test]
	fn test_is_member() {
		assert!(!WorkspacePermission::SuperAdmin.is_member());
		assert!(create_member_permission(BTreeMap::new()).is_member());
	}

	#[test]
	fn test_is_superset_of_super_admin_vs_all() {
		let super_admin = WorkspacePermission::SuperAdmin;
		let member = create_member_permission(BTreeMap::new());

		assert!(super_admin.is_superset_of(&super_admin));
		assert!(super_admin.is_superset_of(&member));
	}

	#[test]
	fn test_is_superset_of_member_vs_super_admin() {
		let super_admin = WorkspacePermission::SuperAdmin;
		let member = create_member_permission(BTreeMap::new());

		assert!(!member.is_superset_of(&super_admin));
	}

	#[test]
	fn test_is_superset_of_include_vs_include_superset() {
		let permission_id = Uuid::new_v4();
		let resource_1 = Uuid::new_v4();
		let resource_2 = Uuid::new_v4();
		let resource_3 = Uuid::new_v4();

		let mut self_perms = BTreeMap::new();
		self_perms.insert(
			permission_id,
			ResourcePermissionType::Include(
				[resource_1, resource_2, resource_3].into_iter().collect(),
			),
		);
		let self_permission = create_member_permission(self_perms);

		let mut other_perms = BTreeMap::new();
		other_perms.insert(
			permission_id,
			ResourcePermissionType::Include([resource_1, resource_2].into_iter().collect()),
		);
		let other_permission = create_member_permission(other_perms);

		assert!(self_permission.is_superset_of(&other_permission));
	}

	#[test]
	fn test_is_superset_of_include_vs_include_not_superset() {
		let permission_id = Uuid::new_v4();
		let resource_1 = Uuid::new_v4();
		let resource_2 = Uuid::new_v4();

		let mut self_perms = BTreeMap::new();
		self_perms.insert(
			permission_id,
			ResourcePermissionType::Include([resource_1].into_iter().collect()),
		);
		let self_permission = create_member_permission(self_perms);

		let mut other_perms = BTreeMap::new();
		other_perms.insert(
			permission_id,
			ResourcePermissionType::Include([resource_1, resource_2].into_iter().collect()),
		);
		let other_permission = create_member_permission(other_perms);

		assert!(!self_permission.is_superset_of(&other_permission));
	}

	#[test]
	fn test_is_superset_of_include_vs_exclude() {
		let permission_id = Uuid::new_v4();
		let resource_1 = Uuid::new_v4();

		let mut self_perms = BTreeMap::new();
		self_perms.insert(
			permission_id,
			ResourcePermissionType::Include([resource_1].into_iter().collect()),
		);
		let self_permission = create_member_permission(self_perms);

		let mut other_perms = BTreeMap::new();
		other_perms.insert(
			permission_id,
			ResourcePermissionType::Exclude([resource_1].into_iter().collect()),
		);
		let other_permission = create_member_permission(other_perms);

		assert!(!self_permission.is_superset_of(&other_permission));
	}

	#[test]
	fn test_is_superset_of_exclude_vs_include_disjoint() {
		let permission_id = Uuid::new_v4();
		let resource_1 = Uuid::new_v4();
		let resource_2 = Uuid::new_v4();

		let mut self_perms = BTreeMap::new();
		self_perms.insert(
			permission_id,
			ResourcePermissionType::Exclude([resource_1].into_iter().collect()),
		);
		let self_permission = create_member_permission(self_perms);

		let mut other_perms = BTreeMap::new();
		other_perms.insert(
			permission_id,
			ResourcePermissionType::Include([resource_2].into_iter().collect()),
		);
		let other_permission = create_member_permission(other_perms);

		assert!(self_permission.is_superset_of(&other_permission));
	}

	#[test]
	fn test_is_superset_of_exclude_vs_include_overlap() {
		let permission_id = Uuid::new_v4();
		let resource_1 = Uuid::new_v4();

		let mut self_perms = BTreeMap::new();
		self_perms.insert(
			permission_id,
			ResourcePermissionType::Exclude([resource_1].into_iter().collect()),
		);
		let self_permission = create_member_permission(self_perms);

		let mut other_perms = BTreeMap::new();
		other_perms.insert(
			permission_id,
			ResourcePermissionType::Include([resource_1].into_iter().collect()),
		);
		let other_permission = create_member_permission(other_perms);

		assert!(!self_permission.is_superset_of(&other_permission));
	}

	#[test]
	fn test_is_superset_of_exclude_vs_exclude_subset() {
		let permission_id = Uuid::new_v4();
		let resource_1 = Uuid::new_v4();
		let resource_2 = Uuid::new_v4();

		let mut self_perms = BTreeMap::new();
		self_perms.insert(
			permission_id,
			ResourcePermissionType::Exclude([resource_1].into_iter().collect()),
		);
		let self_permission = create_member_permission(self_perms);

		let mut other_perms = BTreeMap::new();
		other_perms.insert(
			permission_id,
			ResourcePermissionType::Exclude([resource_1, resource_2].into_iter().collect()),
		);
		let other_permission = create_member_permission(other_perms);

		assert!(self_permission.is_superset_of(&other_permission));
	}

	#[test]
	fn test_is_superset_of_missing_permission_id() {
		let permission_1 = Uuid::new_v4();
		let permission_2 = Uuid::new_v4();
		let resource = Uuid::new_v4();

		let mut self_perms = BTreeMap::new();
		self_perms.insert(
			permission_1,
			ResourcePermissionType::Include([resource].into_iter().collect()),
		);
		let self_permission = create_member_permission(self_perms);

		let mut other_perms = BTreeMap::new();
		other_perms.insert(
			permission_2,
			ResourcePermissionType::Include([resource].into_iter().collect()),
		);
		let other_permission = create_member_permission(other_perms);

		assert!(!self_permission.is_superset_of(&other_permission));
	}

	#[test]
	fn test_has_permission_on_resource_super_admin() {
		let permission_id = Uuid::new_v4();
		let resource_id = Uuid::new_v4();

		assert!(
			WorkspacePermission::SuperAdmin.has_permission_on_resource(resource_id, permission_id)
		);
	}

	#[test]
	fn test_has_permission_on_resource_member_included() {
		let permission_id = Uuid::new_v4();
		let resource_id = Uuid::new_v4();

		let mut perms = BTreeMap::new();
		perms.insert(
			permission_id,
			ResourcePermissionType::Include([resource_id].into_iter().collect()),
		);
		let permission = create_member_permission(perms);

		assert!(permission.has_permission_on_resource(resource_id, permission_id));
	}

	#[test]
	fn test_has_permission_on_resource_member_not_included() {
		let permission_id = Uuid::new_v4();
		let resource_id = Uuid::new_v4();
		let other_resource_id = Uuid::new_v4();

		let mut perms = BTreeMap::new();
		perms.insert(
			permission_id,
			ResourcePermissionType::Include([resource_id].into_iter().collect()),
		);
		let permission = create_member_permission(perms);

		assert!(!permission.has_permission_on_resource(other_resource_id, permission_id));
	}

	#[test]
	fn test_has_permission_on_resource_member_excluded() {
		let permission_id = Uuid::new_v4();
		let resource_id = Uuid::new_v4();

		let mut perms = BTreeMap::new();
		perms.insert(
			permission_id,
			ResourcePermissionType::Exclude([resource_id].into_iter().collect()),
		);
		let permission = create_member_permission(perms);

		assert!(!permission.has_permission_on_resource(resource_id, permission_id));
	}

	#[test]
	fn test_has_permission_on_resource_member_not_excluded() {
		let permission_id = Uuid::new_v4();
		let resource_id = Uuid::new_v4();
		let excluded_resource_id = Uuid::new_v4();

		let mut perms = BTreeMap::new();
		perms.insert(
			permission_id,
			ResourcePermissionType::Exclude([excluded_resource_id].into_iter().collect()),
		);
		let permission = create_member_permission(perms);

		assert!(permission.has_permission_on_resource(resource_id, permission_id));
	}

	#[test]
	fn test_has_permission_on_resource_missing_permission() {
		let permission_id = Uuid::new_v4();
		let resource_id = Uuid::new_v4();

		let permission = create_member_permission(BTreeMap::new());

		assert!(!permission.has_permission_on_resource(resource_id, permission_id));
	}

	#[test]
	fn test_intersect_super_admin_with_super_admin() {
		let result =
			WorkspacePermission::SuperAdmin.intersect_with(&WorkspacePermission::SuperAdmin);
		assert_eq!(result, Some(WorkspacePermission::SuperAdmin));
	}

	#[test]
	fn test_intersect_super_admin_with_non_empty_member() {
		let permission_id = Uuid::new_v4();
		let resource_id = Uuid::new_v4();

		let mut perms = BTreeMap::new();
		perms.insert(
			permission_id,
			ResourcePermissionType::Include([resource_id].into_iter().collect()),
		);
		let member = create_member_permission(perms.clone());

		// SuperAdmin ∩ Member = Member (the narrower side wins).
		assert_eq!(
			WorkspacePermission::SuperAdmin.intersect_with(&member),
			Some(member.clone()),
		);
		assert_eq!(
			member.intersect_with(&WorkspacePermission::SuperAdmin),
			Some(member),
		);
	}

	#[test]
	fn test_intersect_super_admin_with_empty_member_is_none() {
		let empty = create_member_permission(BTreeMap::new());
		assert_eq!(WorkspacePermission::SuperAdmin.intersect_with(&empty), None);
		assert_eq!(empty.intersect_with(&WorkspacePermission::SuperAdmin), None);
	}

	#[test]
	fn test_intersect_include_with_include_overlap() {
		let permission_id = Uuid::new_v4();
		let r1 = Uuid::new_v4();
		let r2 = Uuid::new_v4();
		let r3 = Uuid::new_v4();

		let mut a = BTreeMap::new();
		a.insert(
			permission_id,
			ResourcePermissionType::Include([r1, r2].into_iter().collect()),
		);
		let mut b = BTreeMap::new();
		b.insert(
			permission_id,
			ResourcePermissionType::Include([r2, r3].into_iter().collect()),
		);
		let mut expected = BTreeMap::new();
		expected.insert(
			permission_id,
			ResourcePermissionType::Include([r2].into_iter().collect()),
		);

		assert_eq!(
			create_member_permission(a).intersect_with(&create_member_permission(b)),
			Some(create_member_permission(expected)),
		);
	}

	#[test]
	fn test_intersect_include_with_include_disjoint_drops_entry() {
		let permission_id = Uuid::new_v4();
		let r1 = Uuid::new_v4();
		let r2 = Uuid::new_v4();

		let mut a = BTreeMap::new();
		a.insert(
			permission_id,
			ResourcePermissionType::Include([r1].into_iter().collect()),
		);
		let mut b = BTreeMap::new();
		b.insert(
			permission_id,
			ResourcePermissionType::Include([r2].into_iter().collect()),
		);

		// Only key dropped → empty Member → None.
		assert_eq!(
			create_member_permission(a).intersect_with(&create_member_permission(b)),
			None,
		);
	}

	#[test]
	fn test_intersect_exclude_with_exclude_unions() {
		let permission_id = Uuid::new_v4();
		let r1 = Uuid::new_v4();
		let r2 = Uuid::new_v4();

		let mut a = BTreeMap::new();
		a.insert(
			permission_id,
			ResourcePermissionType::Exclude([r1].into_iter().collect()),
		);
		let mut b = BTreeMap::new();
		b.insert(
			permission_id,
			ResourcePermissionType::Exclude([r2].into_iter().collect()),
		);
		let mut expected = BTreeMap::new();
		expected.insert(
			permission_id,
			ResourcePermissionType::Exclude([r1, r2].into_iter().collect()),
		);

		assert_eq!(
			create_member_permission(a).intersect_with(&create_member_permission(b)),
			Some(create_member_permission(expected)),
		);
	}

	#[test]
	fn test_intersect_include_with_exclude_subtracts() {
		let permission_id = Uuid::new_v4();
		let r1 = Uuid::new_v4();
		let r2 = Uuid::new_v4();
		let r3 = Uuid::new_v4();

		let mut include = BTreeMap::new();
		include.insert(
			permission_id,
			ResourcePermissionType::Include([r1, r2, r3].into_iter().collect()),
		);
		let mut exclude = BTreeMap::new();
		exclude.insert(
			permission_id,
			ResourcePermissionType::Exclude([r2].into_iter().collect()),
		);
		let mut expected = BTreeMap::new();
		expected.insert(
			permission_id,
			ResourcePermissionType::Include([r1, r3].into_iter().collect()),
		);

		assert_eq!(
			create_member_permission(include.clone())
				.intersect_with(&create_member_permission(exclude.clone())),
			Some(create_member_permission(expected.clone())),
		);
		// Reverse argument order is symmetric.
		assert_eq!(
			create_member_permission(exclude).intersect_with(&create_member_permission(include)),
			Some(create_member_permission(expected)),
		);
	}

	#[test]
	fn test_intersect_include_with_exclude_blocks_all_drops_entry() {
		let permission_id = Uuid::new_v4();
		let r1 = Uuid::new_v4();

		let mut include = BTreeMap::new();
		include.insert(
			permission_id,
			ResourcePermissionType::Include([r1].into_iter().collect()),
		);
		let mut exclude = BTreeMap::new();
		exclude.insert(
			permission_id,
			ResourcePermissionType::Exclude([r1].into_iter().collect()),
		);

		assert_eq!(
			create_member_permission(include).intersect_with(&create_member_permission(exclude)),
			None,
		);
	}

	#[test]
	fn test_intersect_drops_permission_id_only_on_one_side() {
		let permission_1 = Uuid::new_v4();
		let permission_2 = Uuid::new_v4();
		let r = Uuid::new_v4();

		let mut a = BTreeMap::new();
		a.insert(
			permission_1,
			ResourcePermissionType::Include([r].into_iter().collect()),
		);
		let mut b = BTreeMap::new();
		b.insert(
			permission_2,
			ResourcePermissionType::Include([r].into_iter().collect()),
		);

		// No overlap → empty → None.
		assert_eq!(
			create_member_permission(a).intersect_with(&create_member_permission(b)),
			None,
		);
	}

	#[test]
	fn test_intersect_workspace_permissions_drops_one_sided_workspaces() {
		let ws_a = Uuid::new_v4();
		let ws_b = Uuid::new_v4();
		let ws_c = Uuid::new_v4();
		let permission_id = Uuid::new_v4();
		let resource_id = Uuid::new_v4();

		let perm = || {
			let mut p = BTreeMap::new();
			p.insert(
				permission_id,
				ResourcePermissionType::Include([resource_id].into_iter().collect()),
			);
			create_member_permission(p)
		};

		let mut left = BTreeMap::new();
		left.insert(ws_a, perm());
		left.insert(ws_b, perm());

		let mut right = BTreeMap::new();
		right.insert(ws_b, perm());
		right.insert(ws_c, perm());

		let out = intersect_workspace_permissions(&left, &right);
		assert_eq!(out.len(), 1);
		assert!(out.contains_key(&ws_b));
	}
}
