use models::api::user::*;

use crate::prelude::*;

pub async fn list_web_logins(
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
	}: AuthenticatedAppRequest<'_, ListWebLoginsRequest>,
) -> Result<AppResponse<ListWebLoginsRequest>, ErrorType> {
	todo!()
}
