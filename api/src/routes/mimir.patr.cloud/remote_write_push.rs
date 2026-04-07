use axum::{
	body::Body,
	extract::{ConnectInfo, State},
	http::Request,
	response::Response,
};
use http::StatusCode;
use prost::Message;
use rustis::client::Client as RedisClient;

use super::models::WriteRequest;
use crate::prelude::*;

/// Handler for Prometheus remote write push requests (`/api/v1/push`).
pub(super) async fn handle_remote_write_push(
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

	// Snappy decompress
	let Ok(decompressed) = snap::raw::Decoder::new().decompress_vec(&body_bytes) else {
		warn!("Failed to snappy-decompress body");
		return Response::builder()
			.status(StatusCode::BAD_REQUEST)
			.body(Body::from("Failed to decompress request body"))
			.unwrap();
	};

	// Decode protobuf
	let Ok(mut write_request) = WriteRequest::decode(decompressed.as_slice()) else {
		warn!("Failed to decode WriteRequest protobuf");
		return Response::builder()
			.status(StatusCode::BAD_REQUEST)
			.body(Body::from("Failed to decode protobuf"))
			.unwrap();
	};

	// Validate and rewrite labels for each timeseries
	let Ok(mut database) = state
		.database
		.acquire()
		.await
		.inspect_err(|err| error!("Failed to acquire database connection: {}", err))
	else {
		return Response::builder()
			.status(StatusCode::INTERNAL_SERVER_ERROR)
			.body(Body::from("Internal Server Error"))
			.unwrap();
	};
	let mut redis_conn = state.redis.clone();

	for ts in &mut write_request.timeseries {
		if let Err(msg) = validate_and_rewrite_labels(
			&mut database,
			&mut redis_conn,
			&mut ts.labels,
			&runner_id,
			&workspace_id,
		)
		.await
		{
			warn!("Label validation failed: {}", msg);
			return Response::builder()
				.status(StatusCode::FORBIDDEN)
				.body(Body::from("Label validation failed"))
				.unwrap();
		}
	}

	// Re-encode → snappy compress → forward
	let encoded = write_request.encode_to_vec();
	let compressed = match snap::raw::Encoder::new().compress_vec(&encoded) {
		Ok(data) => data,
		Err(err) => {
			error!("Failed to snappy-compress: {}", err);
			return Response::builder()
				.status(StatusCode::INTERNAL_SERVER_ERROR)
				.body(Body::from("Internal Server Error"))
				.unwrap();
		}
	};

	super::common::forward_to_mimir(
		&state,
		&method,
		"/api/v1/push",
		&headers,
		&workspace_id,
		compressed,
	)
	.await
}

/// Validate and rewrite labels on a single timeseries.
async fn validate_and_rewrite_labels(
	database: &mut DatabaseConnection,
	redis: &mut RedisClient,
	labels: &mut Vec<super::models::Label>,
	runner_id: &Uuid,
	workspace_id: &Uuid,
) -> Result<(), String> {
	// Validate deployment_id if present
	let mut has_deployment_id = false;
	for label in labels.iter() {
		if label.name == "deployment_id" {
			has_deployment_id = true;
			let deployment_id = label
				.value
				.parse::<Uuid>()
				.map_err(|_| format!("Invalid deployment_id: {}", label.value))?;

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

	// Upsert runner_id and workspace_id with server-derived values
	if let Some(label) = labels.iter_mut().find(|l| l.name == "runner_id") {
		label.value = runner_id.to_string();
	} else {
		labels.push(super::models::Label {
			name: "runner_id".to_string(),
			value: runner_id.to_string(),
		});
	}
	if let Some(label) = labels.iter_mut().find(|l| l.name == "workspace_id") {
		label.value = workspace_id.to_string();
	} else {
		labels.push(super::models::Label {
			name: "workspace_id".to_string(),
			value: workspace_id.to_string(),
		});
	}

	// Force-set source label based on deployment_id presence
	let source_value = if has_deployment_id {
		"deployment"
	} else {
		"runner"
	};
	if let Some(label) = labels.iter_mut().find(|l| l.name == "source") {
		label.value = source_value.to_string();
	} else {
		labels.push(super::models::Label {
			name: "source".to_string(),
			value: source_value.to_string(),
		});
	}

	Ok(())
}
