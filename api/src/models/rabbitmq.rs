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
	MigrationChange,
}

impl Queue {
	pub fn iterator() -> Iter<'static, Queue> {
		static QUEUE: [Queue; 4] = [
			Queue::Infrastructure,
			Queue::Ci,
			Queue::Billing,
			Queue::MigrationChange,
		];
		QUEUE.iter()
	}
}

impl fmt::Display for Queue {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Queue::Infrastructure => write!(f, "infrastructure"),
			Queue::Ci => write!(f, "ci"),
			Queue::Billing => write!(f, "billing"),
			Queue::MigrationChange => write!(f, "migrationChange"),
		}
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "resource", rename_all = "camelCase")]
#[allow(clippy::large_enum_variant, clippy::upper_case_acronyms)]
pub enum InfraRequestData {
	Deployment(DeploymentRequestData),
	BYOC(BYOCData),
	DockerRegistry(DockerRegistryData),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum BYOCData {
	InitKubernetesCluster {
		region_id: Uuid,
		kube_config: String,
		request_id: Uuid,
	},
	CheckClusterForReadiness {
		region_id: Uuid,
		kube_config: String,
		request_id: Uuid,
	},
	GetDigitalOceanKubeconfig {
		api_token: String,
		cluster_id: Uuid,
		region_id: Uuid,
		request_id: Uuid,
	},
	DeleteKubernetesCluster {
		region_id: Uuid,
		kube_config: String,
		request_id: Uuid,
	},
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum DockerRegistryData {
	DeleteDockerImage {
		workspace_id: Uuid,
		repository_name: String,
		digest: String,
		tag: String,
		image_pushed_ip_addr: String,
		request_id: Uuid,
	},
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
#[allow(clippy::large_enum_variant)]
pub enum BillingData {
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
	SendInvoiceForWorkspace {
		workspace: Workspace,
		month: u32,
		year: i32,
		request_id: Uuid,
	},
	RetryPaymentForWorkspace {
		workspace_id: Uuid,
		process_after: DateTime<Utc>,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum MigrationChangeData {
	CheckUserAccountForSpam {
		user_id: Uuid,
		process_after: DateTime<Utc>,
		request_id: Uuid,
	},
}
