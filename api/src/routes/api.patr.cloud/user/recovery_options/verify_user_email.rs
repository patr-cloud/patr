use models::api::user::*;

use crate::prelude::*;

pub async fn verify_user_email(
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
	}: AuthenticatedAppRequest<'_, VerifyUserEmailRequest>,
) -> Result<AppResponse<VerifyUserEmailRequest>, ErrorType> {
	todo!()
}
