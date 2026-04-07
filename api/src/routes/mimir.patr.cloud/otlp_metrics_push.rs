use axum::{
	body::Body,
	extract::{ConnectInfo, State},
	http::Request,
	response::Response,
};
use http::StatusCode;
use opentelemetry_proto::tonic::{
	collector::metrics::v1::ExportMetricsServiceRequest,
	common::v1::{KeyValue, any_value::Value},
};
use prost::Message;
use rustis::client::Client as RedisClient;

use crate::prelude::*;

/// Handler for OTLP metrics push requests (`/otlp/v1/metrics`).
///
/// Supports both JSON (`application/json`) and protobuf
/// (`application/x-protobuf`) payloads. Resource attributes are validated and
/// rewritten using the same typed logic for both content types.
pub(super) async fn handle_otlp_metrics_push(
	State(state): State<AppState>,
	ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
	req: Request<Body>,
) -> Response {
	let Some((runner_id, api_token)) = super::auth::extract_basic_auth(req.headers()) else {
		return Response::builder()
			.status(StatusCode::UNAUTHORIZED)
			.header("WWW-Authenticate", "Basic realm=\"Patr Mimir\"")
			.body(Body::from("Missing or invalid Authorization header"))
			.unwrap();
	};
	let headers = req.headers().clone();
	let method = req.method().clone();
	let content_type = headers
		.get(http::header::CONTENT_TYPE)
		.and_then(|v| v.to_str().ok())
		.unwrap_or("")
		.to_string();

	let (runner_id, workspace_id) =
		match super::auth::authenticate_and_authorize(&state, addr, runner_id, &api_token).await {
			Ok(ids) => ids,
			Err(resp) => return resp,
		};

	let Ok(body_bytes) = axum::body::to_bytes(req.into_body(), super::MAX_BODY_SIZE).await else {
		return Response::builder()
			.status(StatusCode::PAYLOAD_TOO_LARGE)
			.body(Body::from("Request body too large"))
			.unwrap();
	};

	let is_json = content_type.contains("json");
	let is_protobuf = content_type.contains("protobuf");

	if !is_json && !is_protobuf {
		return Response::builder()
			.status(StatusCode::UNSUPPORTED_MEDIA_TYPE)
			.body(Body::from(
				"Unsupported Content-Type. Use application/json or application/x-protobuf",
			))
			.unwrap();
	}

	// Decode the request based on content type
	let mut request: ExportMetricsServiceRequest = if is_json {
		let Ok(req) = serde_json::from_slice(&body_bytes)
			.inspect_err(|err| warn!("Failed to parse OTLP JSON: {}", err))
		else {
			return Response::builder()
				.status(StatusCode::BAD_REQUEST)
				.body(Body::from("Failed to parse JSON"))
				.unwrap();
		};

		req
	} else {
		let Ok(req) = ExportMetricsServiceRequest::decode(&body_bytes[..])
			.inspect_err(|err| warn!("Failed to decode OTLP protobuf: {}", err))
		else {
			return Response::builder()
				.status(StatusCode::BAD_REQUEST)
				.body(Body::from("Failed to decode protobuf"))
				.unwrap();
		};

		req
	};

	// Acquire DB connection for validation
	let mut database = match state.database.acquire().await {
		Ok(conn) => conn,
		Err(err) => {
			error!("Failed to acquire database connection: {}", err);
			return Response::builder()
				.status(StatusCode::INTERNAL_SERVER_ERROR)
				.body(Body::from("Internal Server Error"))
				.unwrap();
		}
	};
	let mut redis_conn = state.redis.clone();

	// Validate and rewrite resource attributes for each resource_metrics entry
	for rm in &mut request.resource_metrics {
		let Some(ref mut resource) = rm.resource else {
			continue;
		};

		if let Err(msg) = validate_and_rewrite_attributes(
			&mut database,
			&mut redis_conn,
			&mut resource.attributes,
			&runner_id,
			&workspace_id,
		)
		.await
		{
			warn!("OTLP attribute validation failed: {}", msg);
			return Response::builder()
				.status(StatusCode::FORBIDDEN)
				.body(Body::from(format!("Attribute validation failed: {msg}")))
				.unwrap();
		}
	}

	// Re-encode using the same format as the incoming request
	let new_body = if is_json {
		serde_json::to_vec(&request).unwrap()
	} else {
		request.encode_to_vec()
	};

	super::common::forward_to_mimir(
		&state,
		&method,
		"/otlp/v1/metrics",
		&headers,
		&workspace_id,
		new_body,
	)
	.await
}

/// Validate and rewrite OTLP resource attributes using typed `KeyValue`.
async fn validate_and_rewrite_attributes(
	database: &mut DatabaseConnection,
	redis: &mut RedisClient,
	attrs: &mut Vec<KeyValue>,
	runner_id: &Uuid,
	workspace_id: &Uuid,
) -> Result<(), String> {
	// Validate deployment_id
	let mut has_deployment_id = false;
	for attr in attrs.iter() {
		if let Some(Value::StringValue(dep_id_str)) =
			attr.value.as_ref().and_then(|v| v.value.as_ref()) &&
			attr.key == "deployment_id"
		{
			has_deployment_id = true;
			let deployment_id = dep_id_str
				.parse::<Uuid>()
				.map_err(|_| format!("Invalid deployment_id: {dep_id_str}"))?;

			let owning_runner =
				super::cache::get_runner_for_deployment(database, redis, &deployment_id)
					.await
					.map_err(|e| format!("DB error: {e}"))?
					.ok_or_else(|| format!("Deployment {deployment_id} not found"))?;

			if owning_runner != *runner_id {
				return Err(format!(
					"Deployment {deployment_id} does not belong to runner {runner_id}"
				));
			}
		}
	}

	// Upsert runner_id, workspace_id with server-derived values; re-check
	// deployment_id presence during rewrite pass
	let mut found_runner_id = false;
	let mut found_workspace_id = false;
	for attr in attrs.iter_mut() {
		if attr.key == "runner_id" {
			attr.value = Some(opentelemetry_proto::tonic::common::v1::AnyValue {
				value: Some(Value::StringValue(runner_id.to_string())),
			});
			found_runner_id = true;
		} else if attr.key == "workspace_id" {
			attr.value = Some(opentelemetry_proto::tonic::common::v1::AnyValue {
				value: Some(Value::StringValue(workspace_id.to_string())),
			});
			found_workspace_id = true;
		} else if attr.key == "deployment_id" {
			has_deployment_id = true;
		}
	}
	let string_attr = |value: String| {
		Some(opentelemetry_proto::tonic::common::v1::AnyValue {
			value: Some(Value::StringValue(value)),
		})
	};
	if !found_runner_id {
		attrs.push(KeyValue {
			key: "runner_id".to_string(),
			value: string_attr(runner_id.to_string()),
		});
	}
	if !found_workspace_id {
		attrs.push(KeyValue {
			key: "workspace_id".to_string(),
			value: string_attr(workspace_id.to_string()),
		});
	}

	// Force-set source attribute based on deployment_id presence
	let source_value = if has_deployment_id {
		"deployment"
	} else {
		"runner"
	};
	let source_attr_value = Some(opentelemetry_proto::tonic::common::v1::AnyValue {
		value: Some(Value::StringValue(source_value.to_string())),
	});
	if let Some(attr) = attrs.iter_mut().find(|a| a.key == "source") {
		attr.value = source_attr_value;
	} else {
		attrs.push(KeyValue {
			key: "source".to_string(),
			value: source_attr_value,
		});
	}

	// Force-set service.name for runner-sourced telemetry
	if !has_deployment_id {
		let service_name = format!("patr.runner.{}", runner_id);
		let service_name_attr = string_attr(service_name);
		if let Some(attr) = attrs.iter_mut().find(|a| a.key == "service.name") {
			attr.value = service_name_attr;
		} else {
			attrs.push(KeyValue {
				key: "service.name".to_string(),
				value: service_name_attr,
			});
		}
	}

	Ok(())
}
