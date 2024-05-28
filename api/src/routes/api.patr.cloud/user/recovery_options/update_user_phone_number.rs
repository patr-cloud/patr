use models::api::user::*;

use crate::prelude::*;

pub async fn update_user_phone_number(
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
	}: AuthenticatedAppRequest<'_, UpdateUserPhoneNumberRequest>,
) -> Result<AppResponse<UpdateUserPhoneNumberRequest>, ErrorType> {
	todo!()
}
