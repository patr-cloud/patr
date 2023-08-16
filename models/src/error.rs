use std::mem;

use axum::{response::IntoResponse, Json};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::utils::{ApiErrorResponse, False};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorType {
	InvalidEmail,
	UserNotFound,
	InvalidPassword,
	MfaRequired,
	MfaOtpInvalid,
	#[serde(with = "serialize_server_error")]
	InternalServerError(anyhow::Error),
}

impl ErrorType {
	pub fn default_status_code(&self) -> StatusCode {
		match self {
			Self::InvalidEmail => StatusCode::BAD_REQUEST,
			Self::UserNotFound => StatusCode::BAD_REQUEST,
			Self::InvalidPassword => StatusCode::UNAUTHORIZED,
			Self::MfaOtpInvalid => StatusCode::UNAUTHORIZED,
			Self::MfaRequired => StatusCode::UNAUTHORIZED,
			Self::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
		}
	}

	pub fn message(&self) -> impl Into<String> {
		match self {
			Self::InvalidEmail => "Invalid email",
			Self::UserNotFound => "No user exists with those credentials",
			Self::InvalidPassword => "Invalid Password",
			Self::MfaRequired => "Two factor authentication required",
			Self::MfaOtpInvalid => "Invalid two factor authentication code",
			Self::InternalServerError(_) => "internal server error",
		}
	}
}

impl PartialEq for ErrorType {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::InternalServerError(_), Self::InternalServerError(_)) => {
				true
			}
			_ => mem::discriminant(self) == mem::discriminant(other),
		}
	}
}

impl Eq for ErrorType {}

mod serialize_server_error {
	use anyhow::Error;
	use serde::{Deserializer, Serializer};

	pub fn serialize<S>(error: &Error, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str("internalServerError")
	}

	pub fn deserialize<'de, D>(
		deserializer: D,
	) -> Result<anyhow::Error, D::Error>
	where
		D: Deserializer<'de>,
	{
		Ok(Error::msg("internalServerError"))
	}
}

impl IntoResponse for ErrorType {
	fn into_response(self) -> axum::response::Response {
		Json(ApiErrorResponse {
			success: False,
			message: self.message().into(),
			error: self,
		})
		.into_response()
	}
}
