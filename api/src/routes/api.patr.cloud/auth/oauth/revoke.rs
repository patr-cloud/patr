use models::api::auth::oauth::*;

use crate::prelude::*;

pub async fn revoke(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: OAuthRevokeTokenPath,
				query: (),
				headers: (),
				body: OAuthRevokeTokenRequestProcessed { token },
			},
		database,
		redis: _,
		client_ip,
		config,
	}: AppRequest<'_, OAuthRevokeTokenRequest>,
) -> Result<AppResponse<OAuthRevokeTokenRequest>, ErrorType> {
	todo!()
}
