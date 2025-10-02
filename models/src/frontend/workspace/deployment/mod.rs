/// The route that shows the deployment creation form
mod create_deployment;
/// The route that shows a single deployment in a workspace
mod deployment_details;
/// The route that lists all deployments in a workspace
mod list_deployments;

pub use self::{create_deployment::*, deployment_details::*, list_deployments::*};
