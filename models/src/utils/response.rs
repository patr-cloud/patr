use axum::http::StatusCode;

use crate::api::ApiEndpoint;

pub struct ApiResponse<E>
where
	E: ApiEndpoint,
{
	pub status_code: StatusCode,
	// pub headers: E::ResponseHeaders,
	pub body: E::ResponseBody,
}

impl<E> ApiResponse<E>
where
	E: ApiEndpoint,
	// E::ResponseHeaders: Default,
{
	pub fn new(status_code: StatusCode, body: E::ResponseBody) -> Self {
		Self {
			status_code,
			// headers: Default::default(),
			body,
		}
	}
}
