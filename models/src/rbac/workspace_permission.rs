use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{prelude::*, rbac::PermissionScope};

/// Represents the kind of permission that is granted on a workspace.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, ts_rs::TS)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkspacePermission {
	/// The user is the super admin of the workspace.
	SuperAdmin,
	/// The user is a member of the workspace.
	Member {
		/// The scope each granted permission applies at.
		#[serde(flatten)]
		permissions: BTreeMap<Uuid, PermissionScope>,
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

	/// Returns true if the current [`WorkspacePermission`] covers everything
	/// `other` grants. With purely additive scopes this is plain per-scope
	/// set containment.
	#[must_use]
	pub fn is_superset_of(&self, other: &WorkspacePermission) -> bool {
		match (self, other) {
			(Self::SuperAdmin, _) => true,
			(Self::Member { .. }, Self::SuperAdmin) => false,
			(
				Self::Member {
					permissions: own_permissions,
				},
				Self::Member {
					permissions: other_permissions,
				},
			) => other_permissions
				.iter()
				.all(|(permission_id, other_scope)| {
					own_permissions
						.get(permission_id)
						.is_some_and(|own_scope| own_scope.is_superset_of(other_scope))
				}),
		}
	}

	/// Returns the intersection of this [`WorkspacePermission`] with `other`
	/// — the permission set allowed by *both* sides. Used at API-token auth
	/// time to clamp a token's declared ceiling by its owner's current
	/// binding-derived permissions.
	///
	/// Returns [`None`] when the intersection is empty, so the caller can
	/// drop the workspace entry entirely.
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
					permissions: own_permissions,
				},
				Self::Member {
					permissions: other_permissions,
				},
			) => {
				let intersected = own_permissions
					.iter()
					.filter_map(|(permission_id, own_scope)| {
						other_permissions
							.get(permission_id)
							.and_then(|other_scope| own_scope.intersect_with(other_scope))
							.map(|scope| (*permission_id, scope))
					})
					.collect::<BTreeMap<_, _>>();

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

	/// Returns true if the current [`WorkspacePermission`] instance has the
	/// given permission on the given resource.
	#[must_use]
	pub fn has_permission_on_resource(&self, resource_id: Uuid, permission_id: Uuid) -> bool {
		match self {
			Self::SuperAdmin => true,
			Self::Member { permissions } => permissions
				.get(&permission_id)
				.is_some_and(|scope| scope.contains_resource(&resource_id)),
		}
	}
}

/// Intersects two workspace-permission maps, keeping only workspaces (and
/// permissions within them) allowed by both sides. Used to clamp an API
/// token's declared ceiling by its owner's current permissions.
#[must_use]
pub fn intersect_workspace_permissions(
	own: &BTreeMap<Uuid, WorkspacePermission>,
	other: &BTreeMap<Uuid, WorkspacePermission>,
) -> BTreeMap<Uuid, WorkspacePermission> {
	own.iter()
		.filter_map(|(workspace_id, own_permission)| {
			other
				.get(workspace_id)
				.and_then(|other_permission| own_permission.intersect_with(other_permission))
				.map(|permission| (*workspace_id, permission))
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use std::collections::{BTreeMap, BTreeSet};

	use super::{WorkspacePermission, intersect_workspace_permissions};
	use crate::{prelude::*, rbac::PermissionScope};

	/// A deterministic test id.
	fn test_id(id: u8) -> Uuid {
		Uuid::parse_str(&format!("{:032x}", u128::from(id))).unwrap()
	}

	/// A member with one permission at the given scope.
	fn member(permission_id: u8, scope: PermissionScope) -> WorkspacePermission {
		WorkspacePermission::Member {
			permissions: BTreeMap::from([(test_id(permission_id), scope)]),
		}
	}

	/// Shorthand for a resource-set scope.
	fn resources(ids: &[u8]) -> PermissionScope {
		PermissionScope::Resources(ids.iter().map(|id| test_id(*id)).collect::<BTreeSet<_>>())
	}

	#[test]
	fn super_admin_is_superset_of_everything() {
		assert!(WorkspacePermission::SuperAdmin.is_superset_of(&WorkspacePermission::SuperAdmin));
		assert!(
			WorkspacePermission::SuperAdmin.is_superset_of(&member(1, PermissionScope::Workspace))
		);
		assert!(
			!member(1, PermissionScope::Workspace).is_superset_of(&WorkspacePermission::SuperAdmin)
		);
	}

	#[test]
	fn member_superset_is_per_permission_containment() {
		assert!(member(1, PermissionScope::Workspace).is_superset_of(&member(1, resources(&[1]))));
		assert!(!member(1, resources(&[1])).is_superset_of(&member(1, PermissionScope::Workspace)));
		assert!(member(1, resources(&[1, 2])).is_superset_of(&member(1, resources(&[2]))));
		assert!(!member(1, resources(&[1])).is_superset_of(&member(2, resources(&[1]))));
	}

	#[test]
	fn has_permission_on_resource_follows_scope() {
		let resource_id = test_id(9);
		assert!(
			WorkspacePermission::SuperAdmin.has_permission_on_resource(resource_id, test_id(1))
		);
		assert!(
			member(1, PermissionScope::Workspace)
				.has_permission_on_resource(resource_id, test_id(1))
		);
		assert!(member(1, resources(&[9])).has_permission_on_resource(resource_id, test_id(1)));
		assert!(!member(1, resources(&[8])).has_permission_on_resource(resource_id, test_id(1)));
		assert!(
			!member(2, PermissionScope::Workspace)
				.has_permission_on_resource(resource_id, test_id(1))
		);
	}

	#[test]
	fn intersect_super_admin_yields_other_side() {
		assert_eq!(
			WorkspacePermission::SuperAdmin.intersect_with(&WorkspacePermission::SuperAdmin),
			Some(WorkspacePermission::SuperAdmin)
		);
		assert_eq!(
			WorkspacePermission::SuperAdmin.intersect_with(&member(1, resources(&[1]))),
			Some(member(1, resources(&[1])))
		);
		assert_eq!(
			WorkspacePermission::SuperAdmin.intersect_with(&WorkspacePermission::Member {
				permissions: BTreeMap::new(),
			}),
			None
		);
	}

	#[test]
	fn intersect_members_is_per_permission() {
		assert_eq!(
			member(1, resources(&[1, 2])).intersect_with(&member(1, resources(&[2, 3]))),
			Some(member(1, resources(&[2])))
		);
		assert_eq!(
			member(1, PermissionScope::Workspace).intersect_with(&member(1, resources(&[3]))),
			Some(member(1, resources(&[3])))
		);
		// Disjoint scopes, or disjoint permissions, share nothing.
		assert_eq!(
			member(1, resources(&[1])).intersect_with(&member(1, resources(&[2]))),
			None
		);
		assert_eq!(
			member(1, PermissionScope::Workspace).intersect_with(&member(2, resources(&[1]))),
			None
		);
	}

	#[test]
	fn intersect_maps_drops_workspaces_not_on_both_sides() {
		let own = BTreeMap::from([
			(test_id(1), member(1, PermissionScope::Workspace)),
			(test_id(2), WorkspacePermission::SuperAdmin),
		]);
		let other = BTreeMap::from([(test_id(1), member(1, resources(&[5])))]);

		let intersected = intersect_workspace_permissions(&own, &other);
		assert_eq!(
			intersected,
			BTreeMap::from([(test_id(1), member(1, resources(&[5])))])
		);
	}
}
