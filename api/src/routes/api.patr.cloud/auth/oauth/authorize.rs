use models::api::auth::oauth::*;

use crate::prelude::*;

pub async fn authorize(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: OAuthAuthorizePath,
				query:
					OAuthAuthorizeQuery {
						response_type,
						client_id,
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
	todo!()
}
