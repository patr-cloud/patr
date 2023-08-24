use axum::{
	response::{IntoResponse, Response},
	Json,
};
use serde::{de::DeserializeOwned, Serialize};

use super::{ApiSuccessResponseBody, True};

pub trait IntoAxumResponse {
	fn into_axum_response(self) -> Response;
}

impl<T> IntoAxumResponse for T
where
	T: Serialize + DeserializeOwned,
{
	fn into_axum_response(self) -> Response {
		Json(ApiSuccessResponseBody {
			success: True,
			response: self,
		})
		.into_response()
	}
}

#[derive(Debug, Default)]
pub struct GenericResponse(pub Response);

impl IntoAxumResponse for GenericResponse {
	fn into_axum_response(self) -> Response {
		self.0
	}
}
