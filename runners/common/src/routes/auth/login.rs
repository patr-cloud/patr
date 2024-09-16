use models::api::auth::*;

use crate::prelude::*;

pub async fn login(
	request: AppRequest<'_, LoginRequest>,
) -> Result<AppResponse<LoginRequest>, ErrorType> {
	let AppRequest {
		config,
		request:
			ProcessedApiRequest {
				path: _,
				query: _,
				headers: _,
				body: LoginRequestProcessed {
					user_id,
					password,
					mfa_otp,
				},
			},
		database: _,
	} = request;
	Err(ErrorType::server_error("Not implemented"))
}
