/// The list of all permissions that can be granted on a Resource.
mod permissions;
/// Represents the type of a resource.
mod resource_type;
/// Represents the kind of permission that is granted on a workspace.
mod workspace_permission;

pub use self::{permissions::*, resource_type::*, workspace_permission::*};
