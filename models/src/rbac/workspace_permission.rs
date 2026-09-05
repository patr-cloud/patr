use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// Represents the kind of permission that is granted on a workspace.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, ts_rs::TS)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkspacePermission {
	/// The user is the super admin of the workspace.
	SuperAdmin,
	/// The user is a member of the workspace.
	Member {
		/// The scopes each granted permission is held at. Every entry is a
		/// resource id; the workspace's own id means the whole workspace.
		#[serde(flatten)]
		permissions: BTreeMap<Uuid, BTreeSet<Uuid>>,
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
	/// `other` grants. Grants are additive, so this is per-permission scope
	/// coverage.
	#[must_use]
	pub fn is_superset_of(&self, other: &WorkspacePermission, workspace_id: Uuid) -> bool {
		match (self, other) {
			(Self::SuperAdmin, _) => true,
			(Self::Member { .. }, Self::SuperAdmin) => false,
			(
				Self::Member { .. },
				Self::Member {
					permissions: other_permissions,
				},
			) => other_permissions
				.iter()
				.all(|(permission_id, other_scopes)| {
					other_scopes.iter().all(|resource_id| {
						self.has_permission_on_resource(workspace_id, *resource_id, *permission_id)
					})
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
	pub fn intersect_with(
		&self,
		other: &WorkspacePermission,
		workspace_id: Uuid,
	) -> Option<WorkspacePermission> {
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
					.filter_map(|(permission_id, own_scopes)| {
						let other_scopes = other_permissions.get(permission_id)?;

						// A grant at the root covers everything the other
						// side names, so the narrower side wins.
						let scopes = if own_scopes.contains(&workspace_id) {
							other_scopes.clone()
						} else if other_scopes.contains(&workspace_id) {
							own_scopes.clone()
						} else {
							own_scopes
								.intersection(other_scopes)
								.copied()
								.collect::<BTreeSet<_>>()
						};

						(!scopes.is_empty()).then(|| (*permission_id, scopes))
					})
					.collect::<BTreeMap<_, _>>();

				(!intersected.is_empty()).then_some(Self::Member {
					permissions: intersected,
				})
			}
		}
	}

	/// Returns true if the current [`WorkspacePermission`] instance has the
	/// given permission on the given resource.
	///
	/// A scope is a resource id, exactly as stored on `role_binding`. The
	/// workspace's own id is the root of the resource tree, so a grant there
	/// covers every resource in the workspace. When resources gain parents,
	/// this is the one place that has to learn to walk them.
	#[must_use]
	pub fn has_permission_on_resource(
		&self,
		workspace_id: Uuid,
		resource_id: Uuid,
		permission_id: Uuid,
	) -> bool {
		match self {
			Self::SuperAdmin => true,
			Self::Member { permissions } => permissions.get(&permission_id).is_some_and(|scopes| {
				scopes.contains(&resource_id) || scopes.contains(&workspace_id)
			}),
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
				.and_then(|other_permission| {
					own_permission.intersect_with(other_permission, *workspace_id)
				})
				.map(|permission| (*workspace_id, permission))
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use std::collections::{BTreeMap, BTreeSet};

	use super::{WorkspacePermission, intersect_workspace_permissions};
	use crate::prelude::*;

	/// A deterministic test id.
	fn test_id(id: u8) -> Uuid {
		Uuid::parse_str(&format!("{:032x}", u128::from(id))).unwrap()
	}

	/// The workspace whose root scope the tests grant at.
	fn workspace() -> Uuid {
		test_id(200)
	}

	/// A member holding one permission at the given scopes.
	fn member(permission_id: u8, scopes: &[Uuid]) -> WorkspacePermission {
		WorkspacePermission::Member {
			permissions: BTreeMap::from([(
				test_id(permission_id),
				scopes.iter().copied().collect::<BTreeSet<_>>(),
			)]),
		}
	}

	/// Shorthand for a set of resource scopes.
	fn resources(ids: &[u8]) -> Vec<Uuid> {
		ids.iter().map(|id| test_id(*id)).collect::<Vec<_>>()
	}

	#[test]
	fn super_admin_is_superset_of_everything() {
		let ws = workspace();
		assert!(
			WorkspacePermission::SuperAdmin.is_superset_of(&WorkspacePermission::SuperAdmin, ws)
		);
		assert!(WorkspacePermission::SuperAdmin.is_superset_of(&member(1, &[ws]), ws));
		assert!(!member(1, &[ws]).is_superset_of(&WorkspacePermission::SuperAdmin, ws));
	}

	#[test]
	fn member_superset_is_per_permission_coverage() {
		let ws = workspace();
		assert!(member(1, &[ws]).is_superset_of(&member(1, &resources(&[1])), ws));
		assert!(!member(1, &resources(&[1])).is_superset_of(&member(1, &[ws]), ws));
		assert!(member(1, &resources(&[1, 2])).is_superset_of(&member(1, &resources(&[2])), ws));
		assert!(!member(1, &resources(&[1])).is_superset_of(&member(2, &resources(&[1])), ws));
	}

	#[test]
	fn has_permission_on_resource_follows_scope() {
		let ws = workspace();
		let resource_id = test_id(9);
		assert!(WorkspacePermission::SuperAdmin.has_permission_on_resource(
			ws,
			resource_id,
			test_id(1)
		));
		// A grant at the workspace root covers every resource under it.
		assert!(member(1, &[ws]).has_permission_on_resource(ws, resource_id, test_id(1)));
		assert!(member(1, &resources(&[9])).has_permission_on_resource(
			ws,
			resource_id,
			test_id(1)
		));
		assert!(!member(1, &resources(&[8])).has_permission_on_resource(
			ws,
			resource_id,
			test_id(1)
		));
		assert!(!member(2, &[ws]).has_permission_on_resource(ws, resource_id, test_id(1)));
	}

	#[test]
	fn intersect_super_admin_yields_other_side() {
		let ws = workspace();
		assert_eq!(
			WorkspacePermission::SuperAdmin.intersect_with(&WorkspacePermission::SuperAdmin, ws),
			Some(WorkspacePermission::SuperAdmin)
		);
		assert_eq!(
			WorkspacePermission::SuperAdmin.intersect_with(&member(1, &resources(&[1])), ws),
			Some(member(1, &resources(&[1])))
		);
		assert_eq!(
			WorkspacePermission::SuperAdmin.intersect_with(
				&WorkspacePermission::Member {
					permissions: BTreeMap::new(),
				},
				ws
			),
			None
		);
	}

	#[test]
	fn intersect_members_is_per_permission() {
		let ws = workspace();
		assert_eq!(
			member(1, &resources(&[1, 2])).intersect_with(&member(1, &resources(&[2, 3])), ws),
			Some(member(1, &resources(&[2])))
		);
		// A root grant narrows to whatever the other side names.
		assert_eq!(
			member(1, &[ws]).intersect_with(&member(1, &resources(&[3])), ws),
			Some(member(1, &resources(&[3])))
		);
		// Disjoint scopes, or disjoint permissions, share nothing.
		assert_eq!(
			member(1, &resources(&[1])).intersect_with(&member(1, &resources(&[2])), ws),
			None
		);
		assert_eq!(
			member(1, &[ws]).intersect_with(&member(2, &resources(&[1])), ws),
			None
		);
	}

	#[test]
	fn intersect_maps_drops_workspaces_not_on_both_sides() {
		let own = BTreeMap::from([
			(test_id(1), member(1, &[test_id(1)])),
			(test_id(2), WorkspacePermission::SuperAdmin),
		]);
		let other = BTreeMap::from([(test_id(1), member(1, &resources(&[5])))]);

		let intersected = intersect_workspace_permissions(&own, &other);
		assert_eq!(
			intersected,
			BTreeMap::from([(test_id(1), member(1, &resources(&[5])))])
		);
	}
}
