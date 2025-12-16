use axum::http::StatusCode;
use models::api::auth::oauth::*;
use rustis::commands::StringCommands;
use serde::Serialize;

use crate::{prelude::*, routes::api_patr_cloud::auth::oauth::token::AuthCodeData};

/// The query parameters for the redirect URL after authorization.
#[derive(Serialize)]
struct RedirectQueryParams {
	code: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	state: Option<String>,
}

pub async fn login(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: OAuthAuthorizePostPath,
				query: (),
				headers: (),
				body:
					OAuthAuthorizePostRequestProcessed {
						client_id,
						redirect_uri,
						scope,
						state,
						code_challenge,
						code_challenge_method,
					},
			},
		database,
		redis,
		client_ip,
		config,
	}: AppRequest<'_, OAuthAuthorizePostRequest>,
) -> Result<AppResponse<OAuthAuthorizePostRequest>, ErrorType> {
	let authorization_code = Uuid::new_v4().to_string();
	info!(
		"Generated authorization code `{}` for client_id `{}`",
		authorization_code, client_id
	);
	let metadata: AuthCodeData = AuthCodeData {
		code_challenge,
		code_challenge_method,
	};

	// Store the authorization code and its metadata in the redis with an expiration
	// time.
	let exp_time = 600;
	let metadata_json = serde_json::to_string(&metadata).map_err(|e| {
		error!("Error serializing authorization code metadata: {}", e);
		ErrorType::server_error(e)
	})?;
	redis
		.setex(
			redis::keys::oauth_authorization_code_prefix(&authorization_code),
			exp_time,
			metadata_json,
		)
		.await
		.inspect_err(|err| {
			error!(
				"Error storing authorization code in Redis for userId : {}",
				// user_data.id,
				err.to_string()
			);
		})
		.map_err(ErrorType::server_error)?;

	let params = RedirectQueryParams {
		code: authorization_code,
		state,
	};

	let query_string = serde_qs::to_string(&params).map_err(|e| {
		error!("Error serializing redirect query params: {}", e);
		ErrorType::server_error(e)
	})?;

	let redirect_url = if let Some(uri) = redirect_uri {
		format!("{}?{}", uri, query_string)
	} else {
		format!("/?{}", query_string)
	};

	AppResponse::builder()
		.body(OAuthAuthorizePostResponse)
		.headers(OAuthAuthorizePostResponseHeaders {
			redirect_url: redirect_url.parse().unwrap(),
		})
		.status_code(StatusCode::FOUND)
		.build()
		.into_result()
}
