/// The models that corresponds to all role RBAC in a workspace on resources
pub mod role;
/// The models that corresponds to all user RBAC in a workspace
pub mod user;

/// The endpoint to get the current permissions of the user in the workspace
mod get_current_permissions;
/// The endpoint to list all the permissions in the workspace
mod list_all_permissions;
/// The endpoint to list all the resource types in the workspace
mod list_all_resource_types;

pub use self::{get_current_permissions::*, list_all_permissions::*, list_all_resource_types::*};
