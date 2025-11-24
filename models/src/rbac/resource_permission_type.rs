use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use strum::EnumDiscriminants;

use crate::prelude::*;

/// Represents the type of permission that is granted on a set of Resource IDs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, EnumDiscriminants, ts_rs::TS)]
#[serde(
	rename_all = "camelCase",
	tag = "permissionType",
	content = "resources"
)]
#[strum_discriminants(
	name(ResourcePermissionTypeDiscriminant),
	derive(strum::Display),
	strum(serialize_all = "snake_case"),
	cfg_attr(
		not(target_arch = "wasm32"),
		derive(sqlx::Type),
		sqlx(type_name = "PERMISSION_TYPE", rename_all = "snake_case")
	),
	doc = "Represents the type of permission that is granted on a set of Resource IDs."
)]
pub enum ResourcePermissionType {
	/// The user is allowed to access a set of Resource IDs. Any other
	/// Resource IDs are by default not allowed.
	Include(
		/// Set of Resource IDs to allow
		BTreeSet<Uuid>,
	),
	/// The user is not allowed to access a set of Resource IDs. Any other
	/// Resource IDs are by default allowed.
	Exclude(
		/// Set of Resource IDs to not allow
		BTreeSet<Uuid>,
	),
}

impl ResourcePermissionType {
	/// Inserts a new resource ID into the current [`ResourcePermissionType`]
	/// instance based on the type of permission.
	pub fn insert(&mut self, resource_id: Uuid) {
		match self {
			Self::Include(resources) | Self::Exclude(resources) => {
				resources.insert(resource_id);
			}
		}
	}

	/// Returns true if the current [`ResourcePermissionType`] instance has
	/// access to a specific resource ID.
	#[must_use]
	pub fn has_resource(&self, resource_id: &Uuid) -> bool {
		match self {
			Self::Include(resources) => resources.contains(resource_id),
			Self::Exclude(resources) => !resources.contains(resource_id),
		}
	}
}
