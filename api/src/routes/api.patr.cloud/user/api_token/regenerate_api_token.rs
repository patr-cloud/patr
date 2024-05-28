use models::api::user::*;

use crate::prelude::*;

pub async fn regenerate_api_token(
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
	}: AuthenticatedAppRequest<'_, RegenerateApiTokenRequest>,
) -> Result<AppResponse<RegenerateApiTokenRequest>, ErrorType> {
	todo!()
}
