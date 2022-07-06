use api_models::models::workspace::ci2::github::EnvVariable;
use serde::{Deserialize, Serialize};

/// Represents a single unit of task which will be triggered based on actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CiFlow {
	/// Indicates the version, so that we can identify the breaking changes
	pub version: String,
	/// Refers to the type of task defined
	#[serde(flatten)]
	pub kind: Kind,
}

/// Indicates the type of task which will be triggered
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind")]
pub enum Kind {
	/// CI pipeline
	Pipeline(Pipeline),
}

/// Pipeline action defines the CI pipeline steps which will be executed.
/// All pipeline steps will start execution under the repository's root
/// directory
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Pipeline {
	/// name of pipeline
	pub name: String,
	/// list of steps to be executed in a single pipeline
	pub steps: Vec<Step>,
}

/// Step represent a single unit of work which will be done in pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Step {
	/// name of the step
	pub name: String,
	/// image can be any linux image hosted on docker hub
	pub image: String,
	/// list of commands to be executed with the given image
	pub commands: Vec<String>,
	/// list of environmental variables that has to be defined while
	/// initializing container
	#[serde(default)]
	pub env: Vec<EnvVariable>,
}
