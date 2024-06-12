use kube::CustomResource;
use models::{
	api::workspace::deployment::{Deployment, DeploymentRunningDetails},
	prelude::WithId,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Patr deployment resource CRD spec. All information about the deployment
/// should be stored here.
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
	kind = "PatrDeployment",
	group = "patr.cloud",
	version = "v1alpha1",
	singular = "PatrDeployment",
	plural = "PatrDeployments",
	namespaced
)]
pub struct PatrDeploymentSpec {
	/// The deployment data with the ID. For more information, see the
	/// documentation of [`Deployment`].
	#[schemars(flatten)]
	pub deployment: WithId<Deployment>,
	/// The running details of the deployment. For more information, see the
	/// documentation of [`DeploymentRunningDetails`].
	#[schemars(flatten)]
	pub running_details: DeploymentRunningDetails,
}
