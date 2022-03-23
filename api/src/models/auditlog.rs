use api_models::{
	models::workspace::infrastructure::deployment::DeploymentStatus,
	utils::Uuid,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeploymentMetadata {
	Create {
		name: String,
		registry: String,
		image_name: String,
		machine_type: Uuid,
		deploy_on_push: bool,
		deploy_on_create: bool,
		horizontal_scale: u16,
		region: Uuid,
		description: String,
	},
	Start {
		name: String,
		deployment_status: DeploymentStatus,
		deploy_on_push: bool,
		description: String,
	},
	Stop {
		name: String,
		deployment_status: DeploymentStatus,
		machine_type: Uuid,
		region: Uuid,
		description: String,
	},
	Delete {
		name: String,
		deployment_status: DeploymentStatus,
		machine_type: Uuid,
		region: Uuid,
		description: String,
	},
	UpdateImage {
		name: String,
		registry: String,
		image_name: String,
		deployment_status: DeploymentStatus,
		machine_type: Uuid,
		deploy_on_push: bool,
		region: Uuid,
		description: String,
	},
}
