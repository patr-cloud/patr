use std::collections::BTreeMap;

use api_models::{
	models::workspace::{
		domain::{DnsRecordValue, DomainNameserverType},
		infrastructure::{
			deployment::{
				Deployment,
				DeploymentRunningDetails,
				EnvironmentVariableValue,
				ExposedPortType,
			},
			managed_urls::ManagedUrlType,
		},
	},
	utils::{StringifiedU16, Uuid},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "action")]
pub enum WorkspaceMetadata {
	Create { name: String },
	Update { name: String },
	Delete {},
}

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
	},
	Stop {},
	Delete {},
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "action")]
pub enum RepositoryMetaData {
	Create {
		repo_name: String,
	},
	PushImage {
		/// user_id of who pushed the image
		pushed_by: Uuid,
		digest: String,
	},
	DeleteImage {
		digest: String,
	},
	Delete {},
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "action")]
pub enum SecretMetaData {
	Create {
		name: String,
	},
	Update {
		#[serde(skip_serializing_if = "Option::is_none")]
		name: Option<String>,
	},
	Delete {},
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "action")]
pub enum DomainMetaData {
	Add {
		domain_name: String,
		domain_nameserver_type: DomainNameserverType,
	},
	Delete {},
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "action")]
pub enum DnsRecordMetaData {
	Add {
		domain_id: Uuid,
		name: String,
		r#type: DnsRecordValue,
		ttl: u32,
	},
	Update {
		#[serde(skip_serializing_if = "Option::is_none")]
		ttl: Option<u32>,
		#[serde(skip_serializing_if = "Option::is_none")]
		target: Option<String>,
		#[serde(skip_serializing_if = "Option::is_none")]
		priority: Option<u16>,
		#[serde(skip_serializing_if = "Option::is_none")]
		proxied: Option<bool>,
	},
	Delete {},
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "action")]
pub enum ManagedDbMetaData {
	Create {
		name: String,
		db_name: String,
		engine: String,
		#[serde(skip_serializing_if = "Option::is_none")]
		version: Option<String>,
		#[serde(skip_serializing_if = "Option::is_none")]
		num_nodes: Option<u64>,
		database_plan: String,
		region: String,
	},
	Delete {},
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "action")]
pub enum ManagedUrlMetaData {
	Create {
		sub_domain: String,
		domain_id: Uuid,
		path: String,
		#[serde(flatten)]
		url_type: ManagedUrlType,
	},
	Update {
		path: String,
		#[serde(flatten)]
		url_type: ManagedUrlType,
	},
	Delete {},
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "action")]
pub enum StaticSiteMetaData {
	Create {
		name: String,
	},
	Update {
		name: Option<String>,
		is_file_updated: bool,
	},
	Start {},
	Stop {},
	Delete {},
}
