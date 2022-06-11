use api_models::{
	models::workspace::infrastructure::{
		deployment::{Deployment, DeploymentRunningDetails},
		static_site::StaticSiteDetails,
	},
	utils::Uuid,
};
use serde::{Deserialize, Serialize};

use super::DeploymentMetadata;
use crate::db::Workspace;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "resource", rename_all = "camelCase")]
#[allow(clippy::large_enum_variant)]
pub enum RequestMessage {
	Deployment(DeploymentRequestData),
	StaticSite(StaticSiteRequestData),
	Database {},
	Workspace(WorkspaceRequestData),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
#[allow(clippy::large_enum_variant)]
pub enum DeploymentRequestData {
	Create {
		workspace_id: Uuid,
		deployment: Deployment,
		image_name: String,
		digest: Option<String>,
		running_details: DeploymentRunningDetails,
		request_id: Uuid,
	},
	UpdateImage {
		workspace_id: Uuid,
		deployment: Deployment,
		image_name: String,
		digest: Option<String>,
		running_details: DeploymentRunningDetails,
		request_id: Uuid,
	},
	Start {
		workspace_id: Uuid,
		deployment: Deployment,
		image_name: String,
		digest: Option<String>,
		running_details: DeploymentRunningDetails,
		user_id: Uuid,
		login_id: Uuid,
		ip_address: String,
		request_id: Uuid,
	},
	Stop {
		workspace_id: Uuid,
		deployment_id: Uuid,
		user_id: Uuid,
		login_id: Uuid,
		ip_address: String,
		request_id: Uuid,
	},
	Update {
		workspace_id: Uuid,
		deployment: Deployment,
		image_name: String,
		digest: Option<String>,
		running_details: DeploymentRunningDetails,
		user_id: Uuid,
		login_id: Uuid,
		ip_address: String,
		metadata: DeploymentMetadata,
		request_id: Uuid,
	},
	Delete {
		workspace_id: Uuid,
		deployment_id: Uuid,
		user_id: Uuid,
		login_id: Uuid,
		ip_address: String,
		request_id: Uuid,
	},
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum StaticSiteRequestData {
	Create {
		workspace_id: Uuid,
		static_site_id: Uuid,
		file: String,
		static_site_details: StaticSiteDetails,
		request_id: Uuid,
	},
	Start {
		workspace_id: Uuid,
		static_site_id: Uuid,
		static_site_details: StaticSiteDetails,
		request_id: Uuid,
	},
	Stop {
		workspace_id: Uuid,
		static_site_id: Uuid,
		request_id: Uuid,
	},
	UploadSite {
		workspace_id: Uuid,
		static_site_id: Uuid,
		file: String,
		request_id: Uuid,
	},
	Delete {
		workspace_id: Uuid,
		static_site_id: Uuid,
		request_id: Uuid,
	},
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "resource", rename_all = "camelCase")]
#[allow(clippy::large_enum_variant)]
pub enum WorkspaceRequestData {
	ProcessWorkspace {
		month: u32,
		year: i32,
	},
	GenerateInvoice {
		month: u32,
		year: i32,
		workspace: Workspace,
	},
	ConfirmPaymentIntent {
		payment_intent_id: String,
	},
}
