use std::{
	error::Error as StdError,
	fmt::{Display, Formatter},
	mem,
};

use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

/// A list of all the possible errors that can be returned by the API
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorType {
	/// The email provided is invalid
	InvalidEmail,
	/// The user was not found
	UserNotFound,
	/// The password provided is invalid
	InvalidPassword,
	/// The user has two factor authentication enabled and it is required
	MfaRequired,
	/// The two factor authentication code provided is invalid
	MfaOtpInvalid,
	/// The parameters sent with the request is invalid. This would ideally not
	/// happen unless there is a bug in the client
	WrongParameters,
	/// The API token provided is invalid
	MalformedApiToken,
	/// The API token provided is not allowed to access the API from the IP
	/// address it is being accessed from
	DisallowedIpAddressForApiToken,
	/// The access token (JWT) provided is malformed
	MalformedAccessToken,
	/// The authentication token provided is not authorized to perform the
	/// requested action
	Unauthorized,
	/// The access token (JWT) provided is invalid
	AuthorizationTokenInvalid,
	/// An internal server error occurred. This should not happen unless there
	/// is a bug in the server
	#[serde(with = "serialize_server_error")]
	InternalServerError(anyhow::Error),
}

impl ErrorType {
	/// Returns the status code that should be used for this error. Note that
	/// this is only the default status code and specific endpoints can override
	/// this if needed
	pub fn default_status_code(&self) -> StatusCode {
		match self {
			Self::InvalidEmail => StatusCode::BAD_REQUEST,
			Self::UserNotFound => StatusCode::BAD_REQUEST,
			Self::InvalidPassword => StatusCode::UNAUTHORIZED,
			Self::MfaOtpInvalid => StatusCode::UNAUTHORIZED,
			Self::MfaRequired => StatusCode::UNAUTHORIZED,
			Self::WrongParameters => StatusCode::BAD_REQUEST,
			Self::MalformedApiToken => StatusCode::BAD_REQUEST,
			Self::DisallowedIpAddressForApiToken => StatusCode::UNAUTHORIZED,
			Self::MalformedAccessToken => StatusCode::BAD_REQUEST,
			Self::Unauthorized => StatusCode::UNAUTHORIZED,
			Self::AuthorizationTokenInvalid => StatusCode::UNAUTHORIZED,
			Self::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
		}
	}

	/// Returns the message that should be used for this error. This is the
	/// message that is user-friendly and can be shown to the user
	pub fn message(&self) -> impl Into<String> {
		match self {
			Self::InvalidEmail => "Invalid email",
			Self::UserNotFound => "No user exists with those credentials",
			Self::InvalidPassword => "Invalid Password",
			Self::MfaRequired => "Two factor authentication required",
			Self::MfaOtpInvalid => "Invalid two factor authentication code",
			Self::WrongParameters => "The parameters sent with that request is invalid",
			Self::MalformedApiToken => "The API token provided is not a valid token",
			Self::DisallowedIpAddressForApiToken => {
				"The API token provided is not allowed from this IP address"
			}
			Self::MalformedAccessToken => "Your access token is invalid. Please login in again",
			Self::Unauthorized => "You are not authorized to perform that action",
			Self::AuthorizationTokenInvalid => "Your access token has expired. Please login again",
			Self::InternalServerError(_) => "internal server error",
		}
	}

	/// Creates an [`ErrorType::InternalServerError`] with the given message
	pub fn server_error(message: impl Display) -> Self {
		Self::InternalServerError(anyhow::anyhow!(message.to_string()))
	}
}

impl PartialEq for ErrorType {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::InternalServerError(_), Self::InternalServerError(_)) => true,
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
			Self::MalformedApiToken => Self::MalformedApiToken,
			Self::DisallowedIpAddressForApiToken => Self::DisallowedIpAddressForApiToken,
			Self::MalformedAccessToken => Self::MalformedAccessToken,
			Self::Unauthorized => Self::Unauthorized,
			Self::AuthorizationTokenInvalid => Self::AuthorizationTokenInvalid,
			Self::InternalServerError(arg0) => {
				Self::InternalServerError(anyhow::anyhow!(arg0.to_string()))
			}
		}
	}
}

impl Display for ErrorType {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.message().into())
	}
}

/// A helper module to serialize and deserialize the internal server error
/// variant of the [`super::ErrorType`] enum
mod serialize_server_error {
	use anyhow::Error;
	use serde::{Deserializer, Serializer};

	/// Converts an
	/// [`ErrorType::InternalServerError`][super::ErrorType::InternalServerError]
	/// into an error field.
	pub fn serialize<S>(_: &Error, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str("internalServerError")
	}

	/// Converts a given error field into an internal server error. Since the
	/// module is only used on the
	/// [`ErrorType::InternalServerError`][super::ErrorType::InternalServerError]
	/// variant, this function will always return an internal server error.
	pub fn deserialize<'de, D>(_: D) -> Result<anyhow::Error, D::Error>
	where
		D: Deserializer<'de>,
	{
		Ok(Error::msg("internalServerError"))
	}
}
