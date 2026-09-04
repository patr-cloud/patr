/// Where a granted permission applies (workspace-wide or a resource set).
/// The list of all permissions that can be granted on a Resource.
mod permissions;
/// Represents the type of permission that is granted on a set of Resource IDs.
mod resource_permission_type;
/// Represents the type of a resource.
mod resource_type;
/// Represents the kind of permission that is granted on a workspace.
mod workspace_permission;

pub use self::{
	permissions::*,
	resource_permission_type::*,
	resource_type::*,
	workspace_permission::*,
};
