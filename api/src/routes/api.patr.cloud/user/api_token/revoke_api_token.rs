use models::api::user::*;

use crate::prelude::*;

pub async fn revoke_api_token(
	AuthenticatedAppRequest {
		request: ProcessedApiRequest {
			path,
			query,
			headers,
			body,
		},
		database,
		redis,
		client_ip,
		user_data,
		config,
	}: AuthenticatedAppRequest<'_, RevokeApiTokenRequest>,
) -> Result<AppResponse<RevokeApiTokenRequest>, ErrorType> {
	todo!()
}
