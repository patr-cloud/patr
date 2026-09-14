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
		user_data,
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
		// Expired, already decided, or never existed — all the same to the
		// user, who just needs to start again from the client.
		debug!("No pending authorization request for `{}`", request_id);
		return Err(ErrorType::ResourceDoesNotExist);
	};

	// The config is the authority on what a client may do; the table only
	// mirrors its display metadata. A client pulled from the config can no
	// longer be consented to, even if its row survives for existing grants.
	let Some(client) = state.config.oauth.clients.get(&request.client_id) else {
		warn!(
			"Authorization request `{}` names client `{}`, which is no longer configured",
			request_id, request.client_id
		);
		return Err(ErrorType::ResourceDoesNotExist);
	};

	// Shown so the user can see where approving will send them. Only the
	// host: the full URI is long, and the host is the part that answers
	// "am I being sent somewhere I recognise".
	let redirect_uri_host = reqwest::Url::parse(&request.redirect_uri)
		.ok()
		.and_then(|url| url.host_str().map(str::to_owned))
		.unwrap_or_else(|| request.redirect_uri.clone());

	// A user who has already approved this client should be told they are
	// reconnecting rather than being shown a first-time decision.
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
		user_data.id as _,
		request.client_id,
	)
	.fetch_one(&mut **database)
	.await?
	.count > 0;

	AppResponse::builder()
		.body(GetConsentRequestResponse {
			client_id: request.client_id.clone(),
			client_name: client.name.clone(),
			client_logo_url: client.logo_url.clone(),
			client_uri: client.client_uri.clone(),
			redirect_uri_host,
			scopes: request
				.scopes
				.iter()
				.map(String::as_str)
				.map(describe_scope)
				.collect(),
			previously_approved,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

/// Turns a scope string into the copy a user is asked to agree to.
///
/// Worded plainly, and honest about the size of what is being granted: with
/// no per-permission ceiling, a token acts with everything its user can do,
/// and the screen has to say so rather than implying the scopes are limits.
fn describe_scope(scope: &str) -> OAuthScopeInfo {
	let (title, description) = match scope {
		"openid" => (
			"Sign you in",
			"Confirm who you are, and act on your behalf with the same access you have.",
		),
		"profile" => ("Your name", "See your first and last name."),
		"email" => (
			"Your email address",
			"See the email address on your account.",
		),
		"offline_access" => (
			"Stay signed in",
			"Keep access when you are not using the app, until you revoke it.",
		),
		other => {
			// Unreachable via `/authorize`, which rejects unknown scopes —
			// but rendering the raw string beats rendering nothing if that
			// ever stops being true.
			warn!("Describing unknown scope `{}`", other);
			(
				"Additional access",
				"An additional permission this app has asked for.",
			)
		}
	};

	OAuthScopeInfo {
		scope: scope.to_owned(),
		title: title.to_owned(),
		description: description.to_owned(),
	}
}
