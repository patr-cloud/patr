use axum::{
	body::Body,
	extract::{ConnectInfo, Path, State},
	http::Request,
	response::Response,
};
use http::StatusCode;

use crate::prelude::*;

/// Handler for OpenBao KV v2 reads
/// (`/v1/secret/data/{workspace_id}/{secret_id}`).
///
/// Runners authenticate with Basic auth, exactly as they do for the log and
/// metric proxies. The value itself is never logged.
pub(super) async fn handle_read_secret(
	State(state): State<AppState>,
	ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
	Path((workspace_id, secret_id)): Path<(Uuid, Uuid)>,
	req: Request<Body>,
) -> Response {
	// Extract auth before any async work to avoid Send issues
	let Some((runner_id, api_token)) = super::auth::extract_basic_auth(req.headers()) else {
		return Response::builder()
			.status(StatusCode::UNAUTHORIZED)
			.header("WWW-Authenticate", "Basic realm=\"Patr Secrets\"")
			.body(Body::from("Missing or invalid Authorization header"))
			.unwrap();
	};

	if let Err(response) = super::auth::authenticate_and_authorize(
		&state,
		addr,
		runner_id,
		&api_token,
		workspace_id,
		secret_id,
	)
	.await
	{
		return response;
	}

	super::common::forward_to_openbao(&state, &workspace_id, &secret_id).await
}
