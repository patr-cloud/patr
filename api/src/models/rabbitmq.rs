use api_models::{
	models::workspace::infrastructure::{
		deployment::{Deployment, DeploymentRunningDetails, DeploymentStatus},
		static_site::{StaticSite, StaticSiteDetails},
	},
	utils::Uuid,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequestMessage {
	pub request_type: RequestType,
	pub request_data: RequestData,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RequestType {
	Update,
	Delete,
}

#[derive(Clone, Debug, Serialize, Deserialize)]

pub enum RequestData {
	Deployment(Box<DeploymentRequestData>),
	StaticSiteRequest(Box<StaticSiteRequestData>),
	DatabaseRequest {},
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DeploymentRequestData {
	Update {
		workspace_id: Uuid,
		deployment: Box<Deployment>,
		full_image: String,
		running_details: DeploymentRunningDetails,
		request_id: Uuid,
	},
	Delete {
		workspace_id: Uuid,
		deployment_id: Uuid,
		request_id: Uuid,
		deployment_status: DeploymentStatus,
	},
}

#[derive(Clone, Debug, Serialize, Deserialize)]

pub enum StaticSiteRequestData {
	Update {
		workspace_id: Uuid,
		static_site: StaticSite,
		static_site_details: StaticSiteDetails,
		request_id: Uuid,
		static_site_status: DeploymentStatus,
	},
	Delete {
		workspace_id: Uuid,
		static_site_id: Uuid,
		request_id: Uuid,
		static_site_status: DeploymentStatus,
	},
}
