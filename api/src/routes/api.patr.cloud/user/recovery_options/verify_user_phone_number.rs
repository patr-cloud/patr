use models::api::user::*;

use crate::prelude::*;

pub async fn verify_user_phone_number(
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
	}: AuthenticatedAppRequest<'_, VerifyUserPhoneNumberRequest>,
) -> Result<AppResponse<VerifyUserPhoneNumberRequest>, ErrorType> {
	todo!()
}
