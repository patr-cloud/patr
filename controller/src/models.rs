use kube::CustomResource;
use models::utils::Uuid;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Our Document custom resource spec
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(kind = "PatrDeployment", group = "kube.rs", version = "v1", namespaced)]
pub struct PatrDeploymentSpec {
	name: String,
}
