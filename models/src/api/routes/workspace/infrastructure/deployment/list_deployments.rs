use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::Deployment;
use crate::{
	utils::{Paginated, Uuid},
	ApiRequest,
};

#[derive(
	Eq,
	Ord,
	Hash,
	Debug,
	Clone,
	Default,
	TypedPath,
	PartialEq,
	Serialize,
	PartialOrd,
	Deserialize,
)]
#[typed_path("/workspace/:workspace_id/infrastructure/deployment")]
pub struct ListDeploymentsPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListDeploymentsRequest {}

impl ApiRequest for ListDeploymentsRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListDeploymentsPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListDeploymentsResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListDeploymentsResponse {
	pub deployments: Vec<Deployment>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{ListDeploymentsRequest, ListDeploymentsResponse};
	use crate::{
		models::workspace::infrastructure::deployment::{
			Deployment,
			DeploymentRegistry,
			DeploymentStatus,
			PatrRegistry,
		},
		utils::{constants, Uuid},
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListDeploymentsRequest {},
			&[
				Token::Struct {
					name: "ListDeploymentsRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListDeploymentsResponse {
				deployments: vec![
					Deployment {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						name: "John Patr's deployment".to_string(),
						registry: DeploymentRegistry::PatrRegistry {
							registry: PatrRegistry,
							repository_id: Uuid::parse_str(
								"2aef18631ded45eb9170dc2166b30867",
							)
							.unwrap(),
						},
						image_tag: "latest".to_string(),
						status: DeploymentStatus::Running,
						region: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30867",
						)
						.unwrap(),
						machine_type: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30867",
						)
						.unwrap(),
						current_live_digest: None,
					},
					Deployment {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
							.unwrap(),
						name: "John Patr's other deployment".to_string(),
						registry: DeploymentRegistry::ExternalRegistry {
							registry: "registry.hub.docker.com".to_string(),
							image_name: "johnpatr/deployment".to_string(),
						},
						image_tag: "non-latest".to_string(),
						status: DeploymentStatus::Deploying,
						region: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30868",
						)
						.unwrap(),
						machine_type: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30868",
						)
						.unwrap(),
						current_live_digest: None,
					},
				],
			},
			&[
				Token::Struct {
					name: "ListDeploymentsResponse",
					len: 1,
				},
				Token::Str("deployments"),
				Token::Seq { len: Some(2) },
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("John Patr's deployment"),
				Token::Str("registry"),
				Token::Str(constants::PATR_REGISTRY),
				Token::Str("repositoryId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("imageTag"),
				Token::Str("latest"),
				Token::Str("status"),
				Token::UnitVariant {
					name: "DeploymentStatus",
					variant: "running",
				},
				Token::Str("region"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("machineType"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("currentLiveDigest"),
				Token::None,
				Token::MapEnd,
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("name"),
				Token::Str("John Patr's other deployment"),
				Token::Str("registry"),
				Token::Str("registry.hub.docker.com"),
				Token::Str("imageName"),
				Token::Str("johnpatr/deployment"),
				Token::Str("imageTag"),
				Token::Str("non-latest"),
				Token::Str("status"),
				Token::UnitVariant {
					name: "DeploymentStatus",
					variant: "deploying",
				},
				Token::Str("region"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("machineType"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("currentLiveDigest"),
				Token::None,
				Token::MapEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListDeploymentsResponse {
				deployments: vec![
					Deployment {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						name: "John Patr's deployment".to_string(),
						registry: DeploymentRegistry::PatrRegistry {
							registry: PatrRegistry,
							repository_id: Uuid::parse_str(
								"2aef18631ded45eb9170dc2166b30867",
							)
							.unwrap(),
						},
						image_tag: "latest".to_string(),
						status: DeploymentStatus::Running,
						region: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30867",
						)
						.unwrap(),
						machine_type: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30867",
						)
						.unwrap(),
						current_live_digest: None,
					},
					Deployment {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
							.unwrap(),
						name: "John Patr's other deployment".to_string(),
						registry: DeploymentRegistry::ExternalRegistry {
							registry: "registry.hub.docker.com".to_string(),
							image_name: "johnpatr/deployment".to_string(),
						},
						image_tag: "non-latest".to_string(),
						status: DeploymentStatus::Deploying,
						region: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30868",
						)
						.unwrap(),
						machine_type: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30868",
						)
						.unwrap(),
						current_live_digest: None,
					},
				],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("deployments"),
				Token::Seq { len: Some(2) },
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("John Patr's deployment"),
				Token::Str("registry"),
				Token::Str(constants::PATR_REGISTRY),
				Token::Str("repositoryId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("imageTag"),
				Token::Str("latest"),
				Token::Str("status"),
				Token::UnitVariant {
					name: "DeploymentStatus",
					variant: "running",
				},
				Token::Str("region"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("machineType"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("currentLiveDigest"),
				Token::None,
				Token::MapEnd,
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("name"),
				Token::Str("John Patr's other deployment"),
				Token::Str("registry"),
				Token::Str("registry.hub.docker.com"),
				Token::Str("imageName"),
				Token::Str("johnpatr/deployment"),
				Token::Str("imageTag"),
				Token::Str("non-latest"),
				Token::Str("status"),
				Token::UnitVariant {
					name: "DeploymentStatus",
					variant: "deploying",
				},
				Token::Str("region"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("machineType"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("currentLiveDigest"),
				Token::None,
				Token::MapEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		)
	}
}
