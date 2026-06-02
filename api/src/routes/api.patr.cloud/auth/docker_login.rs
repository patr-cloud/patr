use axum::http::StatusCode;
use models::api::auth::*;

use crate::prelude::*;

/// The handler to login the user. This will return the access token and the
/// refresh token.
pub async fn docker_login(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: DockerLoginPath,
				query: DockerLoginQueryProcessed { service: _ },
				headers: DockerLoginRequestHeaders {
					user_agent: _,
					authorization,
				},
				body: DockerLoginRequestProcessed {},
			},
		database: _,
		redis: _,
		client_ip: _,
		state: _,
	}: AppRequest<'_, DockerLoginRequest>,
) -> Result<AppResponse<DockerLoginRequest>, ErrorType> {
	trace!("Logging in user to docker: {}", authorization.username());

	if authorization.username() != "patr" {
		return Err(ErrorType::WrongParameters);
	}

	let access_token = authorization.password().to_string();
	let token = access_token.clone();

	AppResponse::builder()
		.body(DockerLoginResponse {
			access_token,
			token,
		})
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
