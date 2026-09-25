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

/// Authenticate the runner's token and authorize it to read the given secret.
///
/// A runner may read any secret in its own workspace: the token needs
/// `Runner::Execute` on the runner it claims to be, and the path has to name
/// that runner's workspace. Every refusal gets the same `403`, and the secret is
/// only looked up once all of that holds, so a caller learns whether a secret
/// exists only if it could read it anyway. A deleted or unknown secret is
/// refused before OpenBao is ever called.
pub(super) async fn authenticate_and_authorize(
	state: &AppState,
	addr: SocketAddr,
	runner_id: Uuid,
	api_token: &str,
	workspace_id: Uuid,
	secret_id: Uuid,
) -> Result<(), Response> {
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
		ClientType::ApiToken,
		&state.config,
		addr.ip(),
		api_token,
	)
	.await
	.map_err(|err| {
		warn!("Authentication failed: {}", err);
		Response::builder()
			.status(StatusCode::UNAUTHORIZED)
			.header("WWW-Authenticate", "Basic realm=\"Patr Secrets\"")
			.body(Body::from("Authentication failed"))
			.unwrap()
	})?;

	// Look up which workspace this runner belongs to
	let runner_workspace_id =
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
				// Refused exactly like a missing permission below, so a token
				// can't be used to probe which runner ids exist.
				warn!("Runner {} not found or deleted", runner_id);
				Response::builder()
					.status(StatusCode::FORBIDDEN)
					.body(Body::from("Access denied"))
					.unwrap()
			})?;

	// Check Runner::Execute permission on this specific runner
	let permission_id = permissions::get_permission_id(
		&mut database,
		Permission::Runner(RunnerPermission::Execute),
	)
	.await;

	if !user_data.has_permission_on_resource(runner_workspace_id, runner_id, permission_id) {
		warn!(
			"User {} does not have Runner::Execute on runner {} in workspace {}",
			user_data.id, runner_id, runner_workspace_id
		);
		return Err(Response::builder()
			.status(StatusCode::FORBIDDEN)
			.body(Body::from("Access denied"))
			.unwrap());
	}

	// The runner can only read secrets of the workspace it belongs to
	if runner_workspace_id != workspace_id {
		warn!(
			"Runner {} belongs to workspace {}, not {}",
			runner_id, runner_workspace_id, workspace_id
		);
		return Err(Response::builder()
			.status(StatusCode::FORBIDDEN)
			.body(Body::from("Access denied"))
			.unwrap());
	}

	let secret = query!(
		r#"
		SELECT
			id
		FROM
			secret
		WHERE
			id = $1 AND
			workspace_id = $2 AND
			deleted IS NULL;
		"#,
		secret_id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut *database)
	.await
	.map_err(|err| {
		error!("Failed to look up secret: {}", err);
		Response::builder()
			.status(StatusCode::INTERNAL_SERVER_ERROR)
			.body(Body::from("Internal Server Error"))
			.unwrap()
	})?;

	if secret.is_none() {
		warn!(
			"Secret {} not found in workspace {}",
			secret_id, workspace_id
		);
		return Err(Response::builder()
			.status(StatusCode::NOT_FOUND)
			.body(Body::from("Secret not found"))
			.unwrap());
	}

	Ok(())
}
