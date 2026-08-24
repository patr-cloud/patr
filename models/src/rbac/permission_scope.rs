use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// Where a granted permission applies. Replaces the include/exclude
/// `ResourcePermissionType`: grants are purely additive — a permission is
/// held workspace-wide (a binding at `scope_id = workspace_id`) or on an
/// explicit set of resources, and evaluation is a union.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ts_rs::TS)]
#[serde(rename_all = "camelCase", tag = "scopeType", content = "resources")]
pub enum PermissionScope {
	/// Every resource in the workspace, including ones created later.
	Workspace,
	/// Only these resources.
	Resources(
		/// The set of resource IDs the permission applies to.
		BTreeSet<Uuid>,
	),
}

impl PermissionScope {
	/// Widens this scope to also cover everything `other` covers.
	pub fn union_with(&mut self, other: &Self) {
		match (&mut *self, other) {
			(Self::Workspace, _) => (),
			(_, Self::Workspace) => *self = Self::Workspace,
			(Self::Resources(own), Self::Resources(other)) => {
				own.extend(other.iter().copied());
			}
		}
	}

	/// Returns true if the scope covers the given resource.
	#[must_use]
	pub fn contains_resource(&self, resource_id: &Uuid) -> bool {
		match self {
			Self::Workspace => true,
			Self::Resources(resources) => resources.contains(resource_id),
		}
	}

	/// Returns true if this scope covers everything `other` covers.
	#[must_use]
	pub fn is_superset_of(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Workspace, _) => true,
			(Self::Resources(_), Self::Workspace) => false,
			(Self::Resources(own), Self::Resources(other)) => own.is_superset(other),
		}
	}

	/// Returns the scope covered by both sides, or [`None`] when they share
	/// nothing.
	#[must_use]
	pub fn intersect_with(&self, other: &Self) -> Option<Self> {
		match (self, other) {
			(Self::Workspace, other) => Some(other.clone()),
			(own, Self::Workspace) => Some(own.clone()),
			(Self::Resources(own), Self::Resources(other)) => {
				let intersection = own.intersection(other).copied().collect::<BTreeSet<_>>();
				if intersection.is_empty() {
					None
				} else {
					Some(Self::Resources(intersection))
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeSet;

	use super::PermissionScope;
	use crate::prelude::*;

	/// A deterministic test id.
	fn test_id(id: u8) -> Uuid {
		Uuid::parse_str(&format!("{:032x}", u128::from(id))).unwrap()
	}

	/// Shorthand for a resource-set scope.
	fn resources(ids: &[u8]) -> PermissionScope {
		PermissionScope::Resources(ids.iter().map(|id| test_id(*id)).collect::<BTreeSet<_>>())
	}

	#[test]
	fn union_workspace_absorbs() {
		let mut scope = resources(&[1]);
		scope.union_with(&PermissionScope::Workspace);
		assert_eq!(scope, PermissionScope::Workspace);

		let mut scope = PermissionScope::Workspace;
		scope.union_with(&resources(&[1]));
		assert_eq!(scope, PermissionScope::Workspace);
	}

	#[test]
	fn union_resources_accumulate() {
		let mut scope = resources(&[1]);
		scope.union_with(&resources(&[2]));
		assert_eq!(scope, resources(&[1, 2]));
	}

	#[test]
	fn contains_resource_matches_scope() {
		let id = test_id(1);
		assert!(PermissionScope::Workspace.contains_resource(&id));
		assert!(resources(&[1]).contains_resource(&id));
		assert!(!resources(&[2]).contains_resource(&id));
	}

	#[test]
	fn superset_is_set_containment() {
		assert!(PermissionScope::Workspace.is_superset_of(&resources(&[1])));
		assert!(!resources(&[1]).is_superset_of(&PermissionScope::Workspace));
		assert!(resources(&[1, 2]).is_superset_of(&resources(&[1])));
		assert!(!resources(&[1]).is_superset_of(&resources(&[1, 2])));
	}

	#[test]
	fn intersection_is_set_intersection() {
		assert_eq!(
			PermissionScope::Workspace.intersect_with(&resources(&[1])),
			Some(resources(&[1]))
		);
		assert_eq!(
			resources(&[1, 2]).intersect_with(&resources(&[2, 3])),
			Some(resources(&[2]))
		);
		assert_eq!(resources(&[1]).intersect_with(&resources(&[2])), None);
	}
}
