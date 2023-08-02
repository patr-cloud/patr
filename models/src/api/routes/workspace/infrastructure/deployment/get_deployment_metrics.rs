use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::{DeploymentMetrics, Interval, Step};
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
#[typed_path(
	"/workspace/:workspace_id/infrastructure/deployment/:deployment_id/metrics"
)]
pub struct GetDeploymentMetricsPath {
	pub workspace_id: Uuid,
	pub deployment_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDeploymentMetricsRequest {
	pub start_time: Option<Interval>,
	pub step: Option<Step>,
}

impl ApiRequest for GetDeploymentMetricsRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetDeploymentMetricsPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = GetDeploymentMetricsResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDeploymentMetricsResponse {
	pub metrics: Vec<DeploymentMetrics>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{GetDeploymentMetricsRequest, GetDeploymentMetricsResponse};
	use crate::{
		models::workspace::infrastructure::deployment::{
			DeploymentMetrics,
			Interval,
			Metric,
			Step,
		},
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetDeploymentMetricsRequest {
				start_time: Some(Interval::Hour),
				step: Some(Step::FiveMinutes),
			},
			&[
				Token::Struct {
					name: "GetDeploymentMetricsRequest",
					len: 2,
				},
				Token::Str("startTime"),
				Token::Some,
				Token::UnitVariant {
					name: "Interval",
					variant: "hour",
				},
				Token::Str("step"),
				Token::Some,
				Token::UnitVariant {
					name: "Step",
					variant: "fiveMinutes",
				},
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetDeploymentMetricsResponse {
				metrics: vec![DeploymentMetrics {
					pod_name: "pod-1".to_string(),
					metrics: vec![Metric {
						timestamp: 0,
						cpu_usage: "0.0017319460740284163".to_string(),
						memory_usage: "404627456".to_string(),
						network_usage_tx: "392.32521395400397".to_string(),
						network_usage_rx: "179.60209690397065".to_string(),
					}],
				}],
			},
			&[
				Token::Struct {
					name: "GetDeploymentMetricsResponse",
					len: 1,
				},
				Token::Str("metrics"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "DeploymentMetrics",
					len: 2,
				},
				Token::Str("podName"),
				Token::Str("pod-1"),
				Token::Str("metrics"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "Metric",
					len: 5,
				},
				Token::Str("timestamp"),
				Token::U64(0),
				Token::Str("cpuUsage"),
				Token::Str("0.0017319460740284163"),
				Token::Str("memoryUsage"),
				Token::Str("404627456"),
				Token::Str("networkUsageTx"),
				Token::Str("392.32521395400397"),
				Token::Str("networkUsageRx"),
				Token::Str("179.60209690397065"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetDeploymentMetricsResponse {
				metrics: vec![DeploymentMetrics {
					pod_name: "pod-1".to_string(),
					metrics: vec![Metric {
						timestamp: 0,
						cpu_usage: "0.0017319460740284163".to_string(),
						memory_usage: "404627456".to_string(),
						network_usage_tx: "392.32521395400397".to_string(),
						network_usage_rx: "179.60209690397065".to_string(),
					}],
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("metrics"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "DeploymentMetrics",
					len: 2,
				},
				Token::Str("podName"),
				Token::Str("pod-1"),
				Token::Str("metrics"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "Metric",
					len: 5,
				},
				Token::Str("timestamp"),
				Token::U64(0),
				Token::Str("cpuUsage"),
				Token::Str("0.0017319460740284163"),
				Token::Str("memoryUsage"),
				Token::Str("404627456"),
				Token::Str("networkUsageTx"),
				Token::Str("392.32521395400397"),
				Token::Str("networkUsageRx"),
				Token::Str("179.60209690397065"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		)
	}
}
