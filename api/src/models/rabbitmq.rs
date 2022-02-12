use api_models::{
	models::workspace::infrastructure::{
		deployment::{Deployment, DeploymentRunningDetails},
		static_site::{StaticSite, StaticSiteDetails},
	},
	utils::Uuid,
};
use serde::{Deserialize, Serialize};

use crate::utils::settings::Settings;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequestMessage {
	pub request_type: RequestType,
	pub request_data: RequestData,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RequestType {
	Create,
	Update,
	Delete,
	Get,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "resource")]
pub enum RequestData {
	Deployment(Box<DeploymentRequestData>),
	StaticSiteRequest(Box<StaticSiteRequestData>),
	DatabaseRequest {},
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum DeploymentRequestData {
	Update {
		workspace_id: Uuid,
		deployment: Box<Deployment>,
		full_image: String,
		running_details: DeploymentRunningDetails,
		config: Box<Settings>,
		request_id: Uuid,
	},
	Delete {
		workspace_id: Uuid,
		deployment_id: Uuid,
		config: Box<Settings>,
		request_id: Uuid,
	},
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum StaticSiteRequestData {
	Update {
		workspace_id: Uuid,
		static_site: StaticSite,
		static_site_details: StaticSiteDetails,
		config: Settings,
		request_id: Uuid,
	},
	Delete {
		workspace_id: Uuid,
		static_site_id: Uuid,
		config: Settings,
		request_id: Uuid,
	},
}
