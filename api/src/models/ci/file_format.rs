use std::{fmt::Display, ops::Deref};

use monostate::MustBe;
use serde::{Deserialize, Serialize};

/// Indicates the type of task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind")]
pub enum CiFlow {
	/// CI pipeline task
	Pipeline(Pipeline),
}

/// Pipeline action defines the CI pipeline steps which will be executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pipeline {
	/// version of pipeline
	pub version: MustBe!("v1"),
	/// name of pipeline
	pub name: LabelName,
	/// list of steps to be executed in a single pipeline
	pub steps: Vec<Step>,
	/// list of services to run in background while executing pipeline
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub services: Vec<Service>,
}

/// Step represent a single unit of work which will be done in pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
	/// name of the step
	pub name: LabelName,
	/// image can be any docker image hosted publicly on docker hub
	pub image: String,
	/// list of commands to be executed with the given image.
	/// these commands will start executing from the repo source
	pub commands: Commands,
	/// list of environmental variables that has to be defined while
	/// initializing container
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub env: Vec<EnvVar>,
}

/// Service represents a background job which will run during pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
	/// name of the service
	pub name: LabelName,
	/// image can be any docker image hosted publicly on docker hub
	pub image: String,
	/// list of commands to be executed with the given image.
	/// these commands will start executing from the repo source
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub commands: Option<Commands>,
	/// list of environmental variables that has to be defined while
	/// initializing container
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub env: Vec<EnvVar>,
	/// TCP port to access this service
	pub port: i32,
}

// Commands which are passed to the docker image
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, untagged)]
pub enum Commands {
	// a single command
	SingleStr(String),
	// list of commands
	VecStr(Vec<String>),
}

impl Display for Commands {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Commands::SingleStr(command) => write!(f, "{}", command),
			Commands::VecStr(commands) => write!(f, "{}", commands.join("\n")),
		}
	}
}

/// Environmental variable which can be used to init containers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnvVar {
	/// key name of the environment variable
	pub name: String,
	/// value of the environment varialbe
	#[serde(flatten)]
	pub value: EnvVarValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnvVarValue {
	Value(String),
	ValueFromSecret(String),
}

/// A wrapped string type used to represent the valid naming
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(try_from = "String", into = "String")]
pub struct LabelName(String);

impl LabelName {
	pub fn is_valid_label_name(name: &str) -> bool {
		// https://kubernetes.io/docs/concepts/overview/working-with-objects/names/#dns-subdomain-names
		// 1. 0 < label.len() <= 63
		// 2. contain only lowercase alphanumeric characters or '-'
		// 3. starts and ends with an alphanumeric character

		!name.is_empty() &&
			name.len() <= 63 &&
			name.bytes()
				.all(|ch| matches!(ch, b'a'..=b'z' | b'0'..=b'9' | b'-')) &&
			!name.starts_with('-') &&
			!name.ends_with('-')
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
}

impl From<LabelName> for String {
	fn from(label: LabelName) -> Self {
		label.0
	}
}

impl TryFrom<String> for LabelName {
	type Error = String;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		if Self::is_valid_label_name(&value) {
			Ok(Self(value))
		} else {
			Err(format!("Invalid label value: {value}"))
		}
	}
}

impl Deref for LabelName {
	type Target = str;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
