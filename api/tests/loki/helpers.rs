use std::time::{SystemTime, UNIX_EPOCH};

use api::routes::loki_patr_cloud::models::{EntryAdapter, PushRequest, StreamAdapter};
use base64::prelude::*;
use models::utils::Uuid;
use prost::Message;

/// Returns the current Unix timestamp in seconds.
fn now_secs() -> i64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap()
		.as_secs() as i64
}

/// Build a Basic Authorization header value for runner auth.
pub fn basic_auth(runner_id: &Uuid, api_token: &str) -> String {
	format!(
		"Basic {}",
		BASE64_STANDARD.encode(format!("{}:{}", runner_id, api_token))
	)
}

/// Build a snappy-compressed protobuf Loki push payload.
pub fn make_loki_push_body(labels: &str, lines: &[&str]) -> Vec<u8> {
	let base_ts = now_secs();
	let entries = lines
		.iter()
		.enumerate()
		.map(|(i, line)| EntryAdapter {
			timestamp: Some(prost_types::Timestamp {
				seconds: base_ts + i as i64,
				nanos: 0,
			}),
			line: line.to_string(),
		})
		.collect();

	let push_request = PushRequest {
		streams: vec![StreamAdapter {
			labels: labels.to_string(),
			entries,
			hash: 0,
		}],
	};

	let encoded = push_request.encode_to_vec();
	snap::raw::Encoder::new()
		.compress_vec(&encoded)
		.expect("snappy compress failed")
}

/// Build an OTLP JSON payload with the given resource attributes.
pub fn make_otlp_json_body(attrs: &[(&str, &str)]) -> Vec<u8> {
	let attributes: Vec<serde_json::Value> = attrs
		.iter()
		.map(|(k, v)| {
			serde_json::json!({
				"key": k,
				"value": { "stringValue": v }
			})
		})
		.collect();

	let body = serde_json::json!({
		"resourceLogs": [{
			"resource": {
				"attributes": attributes
			},
			"scopeLogs": [{
				"scope": {},
				"logRecords": [{
					"timeUnixNano": format!("{}000000000", now_secs()),
					"body": {
						"stringValue": "test log line from OTLP"
					},
					"severityNumber": 9,
					"severityText": "INFO"
				}]
			}]
		}]
	});

	serde_json::to_vec(&body).expect("json serialize failed")
}

/// Build an OTLP protobuf payload with the given resource attributes.
pub fn make_otlp_proto_body(attrs: &[(&str, &str)]) -> Vec<u8> {
	use opentelemetry_proto::tonic::{
		collector::logs::v1::ExportLogsServiceRequest,
		common::v1::{AnyValue, KeyValue, any_value::Value},
		logs::v1::{LogRecord, ResourceLogs, ScopeLogs},
		resource::v1::Resource,
	};

	let attributes: Vec<KeyValue> = attrs
		.iter()
		.map(|(k, v)| KeyValue {
			key: k.to_string(),
			value: Some(AnyValue {
				value: Some(Value::StringValue(v.to_string())),
			}),
			..Default::default()
		})
		.collect();

	let request = ExportLogsServiceRequest {
		resource_logs: vec![ResourceLogs {
			resource: Some(Resource {
				attributes,
				dropped_attributes_count: 0,
				entity_refs: vec![],
			}),
			scope_logs: vec![ScopeLogs {
				scope: None,
				log_records: vec![LogRecord {
					time_unix_nano: now_secs() as u64 * 1_000_000_000,
					body: Some(AnyValue {
						value: Some(Value::StringValue(
							"test log line from OTLP proto".to_string(),
						)),
					}),
					severity_number: 9,
					severity_text: "INFO".to_string(),
					..Default::default()
				}],
				schema_url: String::new(),
			}],
			schema_url: String::new(),
		}],
	};

	request.encode_to_vec()
}
