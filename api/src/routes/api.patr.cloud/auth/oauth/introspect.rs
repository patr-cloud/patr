use models::api::auth::oauth::*;

use crate::prelude::*;

pub async fn introspect(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: OAuthIntrospectPath,
				query: (),
				headers: (),
				body: OAuthIntrospectRequestProcessed { token, client_id },
			},
		database,
		redis: _,
		client_ip,
		config,
	}: AppRequest<'_, OAuthIntrospectRequest>,
) -> Result<AppResponse<OAuthIntrospectRequest>, ErrorType> {
	todo!()
}
