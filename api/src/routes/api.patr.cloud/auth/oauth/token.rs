use models::api::auth::oauth::*;

use crate::prelude::*;

pub async fn token(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: OAuthTokenPath,
				query: (),
				headers: (),
				body:
					OAuthTokenRequestProcessed {
						grant_type,
						client_id,
						code,
						redirect_uri,
						refresh_token,
						code_verifier,
					},
			},
		database,
		redis: _,
		client_ip,
		config,
	}: AppRequest<'_, OAuthTokenRequest>,
) -> Result<AppResponse<OAuthTokenRequest>, ErrorType> {
	todo!()
}
