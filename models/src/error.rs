use std::{error::Error as StdError, mem};

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorType {
	InvalidEmail,
	UserNotFound,
	InvalidPassword,
	MfaRequired,
	MfaOtpInvalid,
	WrongParameters,
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
			Self::WrongParameters => StatusCode::BAD_REQUEST,
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
			Self::WrongParameters => {
				"The parameters sent with that request is invalid"
			}
			Self::InternalServerError(_) => "internal server error",
		}
	}

	pub fn server_error(message: impl Into<String>) -> Self {
		Self::InternalServerError(anyhow::anyhow!(message.into()))
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

impl<Error> From<Error> for ErrorType
where
	Error: StdError + Send + Sync + 'static,
{
	fn from(error: Error) -> Self {
		Self::InternalServerError(error.into())
	}
}

impl Clone for ErrorType {
	fn clone(&self) -> Self {
		match self {
			Self::InvalidEmail => Self::InvalidEmail,
			Self::UserNotFound => Self::UserNotFound,
			Self::InvalidPassword => Self::InvalidPassword,
			Self::MfaRequired => Self::MfaRequired,
			Self::MfaOtpInvalid => Self::MfaOtpInvalid,
			Self::WrongParameters => Self::WrongParameters,
			Self::InternalServerError(arg0) => {
				Self::InternalServerError(anyhow::anyhow!(arg0.to_string()))
			}
		}
	}
}

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
