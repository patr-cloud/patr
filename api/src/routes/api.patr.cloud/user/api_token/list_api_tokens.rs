use models::api::user::*;

use crate::prelude::*;

pub async fn list_api_tokens(
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
	}: AuthenticatedAppRequest<'_, ListApiTokensRequest>,
) -> Result<AppResponse<ListApiTokensRequest>, ErrorType> {
	todo!()
}
