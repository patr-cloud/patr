use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Patr deployment resource CRD spec. All information about the deployment
/// should be stored here.
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
	kind = "PatrDeployment",
	group = "patr.cloud",
	version = "v1alpha1",
	namespaced
)]
pub struct PatrDeploymentSpec {}
