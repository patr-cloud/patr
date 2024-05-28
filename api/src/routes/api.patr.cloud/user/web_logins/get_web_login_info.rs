use models::api::user::*;

use crate::prelude::*;

pub async fn get_web_login_info(
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
	}: AuthenticatedAppRequest<'_, GetWebLoginInfoRequest>,
) -> Result<AppResponse<GetWebLoginInfoRequest>, ErrorType> {
	todo!()
}
