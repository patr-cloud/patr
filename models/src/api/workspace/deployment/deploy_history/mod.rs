use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::prelude::*;

/// The endpoint to delete the deployment history of a deployment
mod delete_deploy_history;
/// The endpoint to list the deployment history of a deployment
mod list_deploy_history;

pub use self::{delete_deploy_history::*, list_deploy_history::*};

/// The deployment history of a deployment. This is a list of the images digests
/// the deployment has ran and the timestamp of when the digest previously ran
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ListableResource)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentDeployHistory {
	/// The images digests the deployment has ran
	pub image_digest: String,
	/// The timestamp of when the digest previously ran
	pub created: OffsetDateTime,
}
