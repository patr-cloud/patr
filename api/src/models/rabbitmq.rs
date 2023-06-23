use std::{fmt, slice::Iter};

use api_models::{
	models::{
		ci::file_format::{Service, Work},
		workspace::infrastructure::deployment::{
			Deployment,
			DeploymentRunningDetails,
		},
	},
	utils::{DateTime, Uuid},
};
use chrono::Utc;
use kube::config::Kubeconfig;
use serde::{Deserialize, Serialize};

use super::ci::EventType;
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
		[Queue::Infrastructure, Queue::Ci, Queue::Billing].iter()
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
#[serde(tag = "resource", rename_all = "camelCase")]
#[allow(clippy::upper_case_acronyms)]
pub enum InfraRequestData {
	Deployment(DeploymentRequestData),
	BYOC(BYOCData),
	DockerRegistry(DockerRegistryData),
	Database(DatabaseRequestData),
	StaticSite(StaticSiteData),
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
pub enum BYOCData {
	InitKubernetesCluster {
		region_id: Uuid,
		kube_config: Kubeconfig,
		tls_cert: String,
		tls_key: String,
		request_id: Uuid,
	},
	CheckClusterForReadiness {
		region_id: Uuid,
		kube_config: Kubeconfig,
		request_id: Uuid,
	},
	GetDigitalOceanKubeconfig {
		api_token: String,
		cluster_id: Uuid,
		region_id: Uuid,
		tls_cert: String,
		tls_key: String,
		request_id: Uuid,
	},
	DeleteKubernetesCluster {
		region_id: Uuid,
		workspace_id: Uuid,
		kube_config: Kubeconfig,
		request_id: Uuid,
	},
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
#[allow(clippy::large_enum_variant)]
pub enum DatabaseRequestData {
	CheckAndUpdateStatus {
		workspace_id: Uuid,
		database_id: Uuid,
		request_id: Uuid,
		password: String,
	},
	ChangeMongoPassword {
		workspace_id: Uuid,
		database_id: Uuid,
		request_id: Uuid,
		password: String,
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
pub enum StaticSiteData {
	CreateStaticSiteUpload {
		static_site_id: Uuid,
		upload_id: Uuid,
		file: String,
		files_length: usize,
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
	CheckAndStartBuild {
		build_id: BuildId,
		services: Vec<Service>,
		work_steps: Vec<Work>,
		event_type: EventType,
		request_id: Uuid,
	},
	BuildStep {
		build_step: BuildStep,
		event_type: EventType,
		request_id: Uuid,
	},
	CleanBuild {
		build_id: BuildId,
		request_id: Uuid,
	},
	SyncRepo {
		workspace_id: Uuid,
		git_provider_id: Uuid,
		request_id: Uuid,
		github_access_token: String,
	},
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum DockerWebhookData {
	NotificationHandler {
		request_body: String,
		content_type: String,
		request_id: Uuid,
	},
}
