use api_models::{
	models::workspace::infrastructure::deployment::{
		Deployment,
		DeploymentRunningDetails,
	},
	utils::Uuid,
};
use k8s_openapi::api::batch::v1::Job;
use serde::{Deserialize, Serialize};

use super::DeploymentMetadata;
use crate::{
	db::Workspace,
	rabbitmq::{BuildId, BuildStepId},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "resource", rename_all = "camelCase")]
#[allow(clippy::large_enum_variant)]
pub enum RequestMessage {
	Deployment(DeploymentRequestData),
	Database {},
	Workspace(WorkspaceRequestData),
	ManagedUrl(ManagedUrlData),
	ContinuousIntegration(CIData),
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
#[allow(clippy::large_enum_variant)]
pub enum WorkspaceRequestData {
	ProcessWorkspaces {
		month: u32,
		year: i32,
		request_id: Uuid,
	},
	GenerateInvoice {
		month: u32,
		year: i32,
		workspace: Workspace,
		request_id: Uuid,
	},
	ConfirmPaymentIntent {
		payment_intent_id: String,
		workspace_id: Uuid,
		request_id: Uuid,
	},
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
#[allow(clippy::large_enum_variant)]
pub enum ManagedUrlData {
	Create {
		managed_url_id: Uuid,
		workspace_id: Uuid,
		request_id: Uuid,
	},
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
#[allow(clippy::large_enum_variant)]
pub enum CIData {
	InitRepo {
		build_step_id: BuildStepId,
		job: Job,
		request_id: Uuid,
	},
	CreateBuildStep {
		build_step_id: BuildStepId,
		job: Job,
		request_id: Uuid,
	},
	UpdateBuildStepStatus {
		build_step_id: BuildStepId,
		request_id: Uuid,
	},
	CleanBuild {
		build_id: BuildId,
		request_id: Uuid,
	},
}
