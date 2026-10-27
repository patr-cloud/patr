use axum::http::StatusCode;
use models::api::auth::oauth::*;
use rustis::commands::StringCommands;

use crate::{models::oauth::types::OAuthAuthorizationRequest, prelude::*};

/// Renders the pending authorization request behind a consent screen.
///
/// Reads without consuming, so a reload or a server-rendered first paint
/// followed by hydration both work. Only the submit endpoint resolves the
/// request.
pub async fn get_consent_request(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetConsentRequestPath { request_id },
				query: (),
				headers:
					GetConsentRequestRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetConsentRequestRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		actor_data,
		state,
	}: AuthenticatedAppRequest<'_, GetConsentRequestRequest>,
) -> Result<AppResponse<GetConsentRequestRequest>, ErrorType> {
	trace!("Reading OAuth consent request `{}`", request_id);

	let parked: Option<String> = redis
		.get(redis::keys::oauth_authorization_request(&request_id))
		.await?;

	let Some(request) = parked
		.as_deref()
		.and_then(|payload| serde_json::from_str::<OAuthAuthorizationRequest>(payload).ok())
	else {
		debug!("No pending authorization request for `{}`", request_id);
		return Err(ErrorType::ResourceDoesNotExist);
	};

	let Some(client) = state.config.oauth.clients.get(&request.client_id) else {
		warn!(
			"Authorization request `{}` names client `{}`, which is no longer configured",
			request_id, request.client_id
		);
		return Err(ErrorType::ResourceDoesNotExist);
	};

	let redirect_uri_host = reqwest::Url::parse(&request.redirect_uri)
		.ok()
		.and_then(|url| url.host_str().map(str::to_owned))
		.unwrap_or_else(|| request.redirect_uri.clone());

	let previously_approved = query!(
		r#"
		SELECT
			COUNT(*) AS "count!"
		FROM
			oauth_login
		WHERE
			user_id = $1 AND
			client_id = $2 AND
			revoked IS NULL;
		"#,
		actor_data.id,
		request.client_id,
	)
	.fetch_one(&mut **database)
	.await?
	.count >
		0;

	AppResponse::builder()
		.body(GetConsentRequestResponse {
			client_id: request.client_id.clone(),
			client_name: client.name.clone(),
			client_logo_url: client.logo_url.clone(),
			client_uri: client.client_uri.clone(),
			redirect_uri_host,
			scopes: request.scopes,
			previously_approved,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
