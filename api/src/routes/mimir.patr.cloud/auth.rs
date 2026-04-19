use std::net::SocketAddr;

use axum::{body::Body, response::Response};
use base64::prelude::*;
use http::StatusCode;

use crate::{models::permissions, prelude::*};

/// Extract runner ID and API token from HTTP Basic Auth header.
/// Returns `(runner_id, api_token)` or None if invalid.
pub(super) fn extract_basic_auth(headers: &http::HeaderMap) -> Option<(Uuid, String)> {
	let auth_header = headers.get(http::header::AUTHORIZATION)?;
	let auth_str = auth_header.to_str().ok()?;
	let encoded = auth_str.strip_prefix("Basic ")?;
	let decoded = String::from_utf8(BASE64_STANDARD.decode(encoded).ok()?).ok()?;
	let (runner_id_str, api_token) = decoded.split_once(':')?;
	let runner_id = runner_id_str.parse::<Uuid>().ok()?;
	Some((runner_id, api_token.to_string()))
}

/// Shared authentication and permission checking for both Prometheus remote
/// write and OTLP handlers. Takes pre-extracted credentials. Returns
/// `(runner_id, workspace_id)` on success, or an error Response.
pub(super) async fn authenticate_and_authorize(
	state: &AppState,
	addr: SocketAddr,
	runner_id: Uuid,
	api_token: &str,
) -> Result<(Uuid, Uuid), Response> {
	let mut database = state.database.acquire().await.map_err(|err| {
		error!("Failed to acquire database connection: {}", err);
		Response::builder()
			.status(StatusCode::INTERNAL_SERVER_ERROR)
			.body(Body::from("Internal Server Error"))
			.unwrap()
	})?;
	let mut redis_conn = state.redis.clone();

	// Authenticate the API token
	let user_data = permissions::get_user_data_for_token(
		&mut database,
		&mut redis_conn,
		&state.config,
		addr.ip(),
		api_token,
	)
	.await
	.map_err(|err| {
		warn!("Authentication failed: {}", err);
		Response::builder()
			.status(StatusCode::UNAUTHORIZED)
			.header("WWW-Authenticate", "Basic realm=\"Patr Mimir\"")
			.body(Body::from("Authentication failed"))
			.unwrap()
	})?;

	// Mimir push is only allowed from runners (service accounts), not from users
	if user_data.client_type != ClientType::ServiceAccount {
		warn!(
			"Mimir push attempted by non-service-account client: {:?}",
			user_data.client_type
		);
		return Err(Response::builder()
			.status(StatusCode::FORBIDDEN)
			.body(Body::from(
				"Mimir push is only allowed from service accounts",
			))
			.unwrap());
	}

	// Look up which workspace this runner belongs to
	let workspace_id =
		super::cache::get_workspace_for_runner(&mut database, &mut redis_conn, &runner_id)
			.await
			.map_err(|err| {
				error!("Failed to look up runner workspace: {}", err);
				Response::builder()
					.status(StatusCode::INTERNAL_SERVER_ERROR)
					.body(Body::from("Internal Server Error"))
					.unwrap()
			})?
			.ok_or_else(|| {
				warn!("Runner {} not found or deleted", runner_id);
				Response::builder()
					.status(StatusCode::UNAUTHORIZED)
					.body(Body::from("Runner not found"))
					.unwrap()
			})?;

	// Check Runner::Execute permission on this specific runner
	let permission_id = permissions::get_permission_id(
		&mut database,
		Permission::Runner(RunnerPermission::Execute),
	)
	.await;

	if !user_data.has_permission_on_resource(workspace_id, runner_id, permission_id) {
		warn!(
			"User {} does not have Runner::Execute on runner {} in workspace {}",
			user_data.id, runner_id, workspace_id
		);
		return Err(Response::builder()
			.status(StatusCode::FORBIDDEN)
			.body(Body::from(
				"Access denied: missing Runner::Execute permission",
			))
			.unwrap());
	}

	Ok((runner_id, workspace_id))
}
