use std::collections::BTreeMap;

use api_models::{
	models::workspace::infrastructure::deployment::{
		Deployment,
		DeploymentRunningDetails,
		EnvironmentVariableValue,
		ExposedPortType,
	},
	utils::{StringifiedU16, Uuid},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "action")]
pub enum DeploymentMetadata {
	Create {
		deployment: Deployment,
		running_details: DeploymentRunningDetails,
	},
	Start {},
	Update {
		#[serde(skip_serializing_if = "Option::is_none")]
		name: Option<String>,
		#[serde(skip_serializing_if = "Option::is_none")]
		region: Option<Uuid>,
		#[serde(skip_serializing_if = "Option::is_none")]
		machine_type: Option<Uuid>,
		#[serde(skip_serializing_if = "Option::is_none")]
		deploy_on_push: Option<bool>,
		#[serde(skip_serializing_if = "Option::is_none")]
		min_horizontal_scale: Option<u16>,
		#[serde(skip_serializing_if = "Option::is_none")]
		max_horizontal_scale: Option<u16>,
		#[serde(skip_serializing_if = "Option::is_none")]
		ports: Option<BTreeMap<StringifiedU16, ExposedPortType>>,
		#[serde(skip_serializing_if = "Option::is_none")]
		environment_variables:
			Option<BTreeMap<String, EnvironmentVariableValue>>,
		#[serde(skip_serializing_if = "Option::is_none")]
		health_check_port: Option<i32>,
		#[serde(skip_serializing_if = "Option::is_none")]
		health_check_path: Option<String>,
	},
	Stop {},
	Delete {},
	UpdateImage {
		digest: String,
	},
}
