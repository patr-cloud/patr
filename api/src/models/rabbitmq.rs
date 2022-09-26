use std::{fmt, slice::Iter};

use api_models::{
	models::workspace::infrastructure::deployment::{
		Deployment,
		DeploymentRunningDetails,
	},
	utils::{DateTime, Uuid},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::DeploymentMetadata;
use crate::{
	db::Workspace,
	rabbitmq::{BuildId, BuildStep},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Queue {
	Infrastructure,
	Ci,
	Billing,
}

impl Queue {
	pub fn iterator() -> Iter<'static, Queue> {
		static QUEUE: [Queue; 3] =
			[Queue::Infrastructure, Queue::Ci, Queue::Billing];
		QUEUE.iter()
	}
}

impl fmt::Display for Queue {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Queue::Infrastructure => write!(f, "infrastructure"),
			Queue::Ci => write!(f, "ci"),
			Queue::Billing => write!(f, "billing"),
		}
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
#[allow(clippy::large_enum_variant)]
pub enum DeploymentRequestData {
	CheckAndUpdateStatus {
		workspace_id: Uuid,
		deployment_id: Uuid,
	},
	UpdateImage {
		workspace_id: Uuid,
		deployment: Deployment,
		image_name: String,
		digest: String,
		running_details: DeploymentRunningDetails,
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
	AttemptToChargeWorkspace {
		workspace: Workspace,
		process_after: DateTime<Utc>,
		total_bill: f64,
		amount_due: f64,
		month: u32,
		year: i32,
		request_id: Uuid,
	},
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
#[allow(clippy::large_enum_variant)]
pub enum CIData {
	BuildStep {
		build_step: BuildStep,
		request_id: Uuid,
	},
	CancelBuild {
		build_id: BuildId,
		request_id: Uuid,
	},
	CleanBuild {
		build_id: BuildId,
		request_id: Uuid,
	},
}
