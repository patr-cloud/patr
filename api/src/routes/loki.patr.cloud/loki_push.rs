use axum::{
	body::Body,
	extract::{ConnectInfo, State},
	http::Request,
	response::Response,
};
use http::StatusCode;
use prost::Message;

use super::models::PushRequest;
use crate::prelude::*;

/// Handler for Loki protobuf push requests (`/loki/api/v1/push`).
pub(super) async fn handle_loki_push(
	State(state): State<AppState>,
	ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
	req: Request<Body>,
) -> Response {
	// Extract auth before any async work to avoid Send issues
	let Some((runner_id, api_token)) = super::auth::extract_basic_auth(req.headers()) else {
		return Response::builder()
			.status(StatusCode::UNAUTHORIZED)
			.header("WWW-Authenticate", "Basic realm=\"Patr Loki\"")
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

	// Buffer the body (limit to MAX_BODY_SIZE)
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
	let Ok(mut push_request) = PushRequest::decode(decompressed.as_slice()) else {
		warn!("Failed to decode PushRequest protobuf");
		return Response::builder()
			.status(StatusCode::BAD_REQUEST)
			.body(Body::from("Failed to decode protobuf"))
			.unwrap();
	};

	// Validate and rewrite labels for each stream
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

	for stream in &mut push_request.streams {
		let Ok(new_labels) = super::common::validate_and_rewrite_labels(
			&mut database,
			&mut redis_conn,
			&stream.labels,
			&runner_id,
			&workspace_id,
		)
		.await
		.inspect_err(|err| {
			warn!("Label validation failed: {}", err);
		}) else {
			return Response::builder()
				.status(StatusCode::FORBIDDEN)
				.body(Body::from("Label validation failed"))
				.unwrap();
		};
		stream.labels = new_labels;
	}

	// Re-encode → snappy compress → forward
	let encoded = push_request.encode_to_vec();
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

	super::common::forward_to_loki(
		&state,
		&method,
		"/loki/api/v1/push",
		&headers,
		&workspace_id,
		compressed,
	)
	.await
}
