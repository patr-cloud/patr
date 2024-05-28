use models::api::user::*;

use crate::prelude::*;

pub async fn delete_web_login(
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
	}: AuthenticatedAppRequest<'_, DeleteWebLoginRequest>,
) -> Result<AppResponse<DeleteWebLoginRequest>, ErrorType> {
	todo!()
}
