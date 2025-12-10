use models::api::user::*;

use crate::prelude::*;

pub async fn update_user_email(
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
		state,
	}: AuthenticatedAppRequest<'_, UpdateUserEmailRequest>,
) -> Result<AppResponse<UpdateUserEmailRequest>, ErrorType> {
	todo!()
}
