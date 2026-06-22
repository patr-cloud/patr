use axum::http::StatusCode;
use models::api::auth::*;

use crate::{models::permissions, prelude::*};

/// The handler to login the user. This will return the access token and the
/// refresh token.
pub async fn docker_login(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: DockerLoginPath,
				query: DockerLoginQuery { service: _ },
				headers: DockerLoginRequestHeaders {
					user_agent: _,
					authorization,
				},
				body: DockerLoginRequestProcessed {},
			},
		database,
		redis,
		client_ip,
		state,
	}: AppRequest<'_, DockerLoginRequest>,
) -> Result<AppResponse<DockerLoginRequest>, ErrorType> {
	trace!("Logging in user to docker: {}", authorization.username());

	if authorization.username() != "patr" {
		return Err(ErrorType::WrongParameters);
	}

	// Validate the token before handing it back as a bearer credential. Without
	// this, `docker login` "succeeds" with any string and the failure only
	// surfaces later at push time as an opaque registry error.
	permissions::get_user_data_for_token(
		database,
		redis,
		ClientType::ApiToken,
		&state.config,
		client_ip,
		authorization.password(),
	)
	.await?;

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
