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
/// `Runner::Execute` on the runner it claims to be, and the secret has to live
/// in that same workspace. The secret is checked here so a deleted or foreign
/// id is refused before OpenBao is ever called.
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

	// Resolve the runner and the secret in one go. No row means one of them is
	// gone, so the caller gets the same answer either way.
	let resolved = query!(
		r#"
		SELECT
			runner.workspace_id AS "runner_workspace_id: Uuid",
			secret.workspace_id AS "secret_workspace_id: Uuid"
		FROM
			runner,
			secret
		WHERE
			runner.id = $1 AND
			runner.deleted IS NULL AND
			secret.id = $2 AND
			secret.deleted IS NULL;
		"#,
		runner_id as _,
		secret_id as _,
	)
	.fetch_optional(&mut *database)
	.await
	.map_err(|err| {
		error!("Failed to look up runner and secret: {}", err);
		Response::builder()
			.status(StatusCode::INTERNAL_SERVER_ERROR)
			.body(Body::from("Internal Server Error"))
			.unwrap()
	})?
	.ok_or_else(|| {
		warn!("Runner {} or secret {} not found", runner_id, secret_id);
		Response::builder()
			.status(StatusCode::NOT_FOUND)
			.body(Body::from("Secret not found"))
			.unwrap()
	})?;

	// The secret has to live in the workspace the request names
	if resolved.secret_workspace_id != workspace_id {
		warn!(
			"Secret {} not found in workspace {}",
			secret_id, workspace_id
		);
		return Err(Response::builder()
			.status(StatusCode::NOT_FOUND)
			.body(Body::from("Secret not found"))
			.unwrap());
	}

	// The runner can only read secrets of the workspace it belongs to
	if resolved.runner_workspace_id != workspace_id {
		warn!(
			"Runner {} belongs to workspace {}, not {}",
			runner_id, resolved.runner_workspace_id, workspace_id
		);
		return Err(Response::builder()
			.status(StatusCode::FORBIDDEN)
			.body(Body::from("Access denied: runner is in another workspace"))
			.unwrap());
	}

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

	Ok(())
}
