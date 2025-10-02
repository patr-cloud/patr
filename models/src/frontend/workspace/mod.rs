/// The route to create a new workspace
mod create_workspace;

/// All routes that have to do with a deployment
pub mod deployment;

pub use self::create_workspace::*;
