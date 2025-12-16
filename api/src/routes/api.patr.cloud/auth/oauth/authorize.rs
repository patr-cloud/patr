use axum::http::StatusCode;
use models::api::auth::oauth::*;
use serde::Serialize;

use crate::prelude::*;

/// The query parameters the user will be redirected to frontend login page.
#[derive(Serialize)]
struct LoginQueryParams {
	response_type: String,
	client_id: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	redirect_uri: Option<String>,
	scope: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	state: Option<String>,
	code_challenge: String,
	code_challenge_method: String,
}

pub async fn authorize(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: OAuthAuthorizePath,
				query:
					OAuthAuthorizeQuery {
						response_type,
						client_id,
						client_secret,
						redirect_uri,
						scope,
						state,
						code_challenge,
						code_challenge_method,
					},
				headers: (),
				body: OAuthAuthorizeRequestProcessed,
			},
		database,
		redis: _,
		client_ip,
		config,
	}: AppRequest<'_, OAuthAuthorizeRequest>,
) -> Result<AppResponse<OAuthAuthorizeRequest>, ErrorType> {
	if response_type != OAuthAuthorizeResponseType::AuthorizationCode {
		return Err(ErrorType::OAuthInvalidResponseType);
	}
	let client = query!(
		r#"
		SELECT
			secret, redirect_uri
		FROM
			oauth_clients
		WHERE
			id = $1;
		"#,
		client_id
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::OAuthInvalidClient)
	.inspect_err(|_| {
		info!("Could not find a row with the given client_id");
	})?;

	if client.secret != client_secret {
		return Err(ErrorType::OAuthInvalidClient);
	}
	if client.redirect_uri != redirect_uri {
		return Err(ErrorType::OAuthInvalidClient);
	}

	let params = LoginQueryParams {
		response_type: "code".to_string(),
		client_id,
		redirect_uri,
		scope,
		state,
		code_challenge,
		code_challenge_method: code_challenge_method.to_string(),
	};

	let query_string = serde_qs::to_string(&params).map_err(|e| {
		error!("Error serializing query params: {}", e);
		ErrorType::server_error(e)
	})?;

	let login = format!("http://localhost:3001/login?{}", query_string);

	AppResponse::builder()
		.body(OAuthAuthorizeResponse)
		.headers(OAuthAuthorizeResponseHeaders {
			redirect_url: login.parse().unwrap(),
		})
		.status_code(StatusCode::FOUND)
		.build()
		.into_result()
}
