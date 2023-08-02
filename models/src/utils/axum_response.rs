use axum::{
	response::{IntoResponse, Response},
	Json,
};
use serde::Serialize;

pub trait IntoAxumResponse {
	fn into_response(self) -> Response;
}

impl<T> IntoAxumResponse for T
where
	T: Serialize,
{
	fn into_response(self) -> Response {
        Json(self).into_response()
	}
}

#[derive(Debug, Default)]
pub struct GenericResponse(pub Response);

impl IntoAxumResponse for GenericResponse {
	fn into_response(self) -> Response {
		self.0
	}
}
