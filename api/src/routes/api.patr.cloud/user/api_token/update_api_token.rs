use models::api::user::*;

use crate::prelude::*;

pub async fn update_api_token(
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
	}: AuthenticatedAppRequest<'_, UpdateApiTokenRequest>,
) -> Result<AppResponse<UpdateApiTokenRequest>, ErrorType> {
	todo!()
}
