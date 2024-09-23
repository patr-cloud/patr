use std::{error::Error as StdError, fmt::Display, str::FromStr};

use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, IntoEnumIterator};

use crate::prelude::*;

/// A list of all the possible errors that can be returned by the API
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Display, EnumIter,
)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
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
	/// The user already has two factor authentication enabled, and tried
	/// enabling it
	MfaAlreadyActive,
	/// The user does not have two factor authentication enabled, and tried
	/// disabling it
	MfaAlreadyInactive,
	/// The tag does not exist
	TagNotFound,
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
	/// The refresh token provided is malformed
	MalformedRefreshToken,
	/// The authentication token provided is not authorized to perform the
	/// requested action
	Unauthorized,
	/// The access token (JWT) provided is invalid
	AuthorizationTokenInvalid,
	/// The username provided is not available. It is being used by another
	/// account
	UsernameUnavailable,
	/// The email provided is not available. It is being used by another account
	EmailUnavailable,
	/// The phone number provided is not available. It is being used by another
	/// account
	PhoneUnavailable,
	/// The reset token used to reset the given user's password is invalid.
	InvalidPasswordResetToken,
	/// The resource that the user is trying to access does not exist.
	ResourceDoesNotExist,
	/// The resource already exists
	ResourceAlreadyExists,
	/// The resource is currently in use
	ResourceInUse,
	/// A workspace with that name cannot be created because it already exists
	WorkspaceNameAlreadyExists,
	/// Tried to delete a workspace that has resources in it
	WorkspaceNotEmpty,
	/// Volume of a deployment cannot be reduced
	CannotReduceVolumeSize,
	/// Cannot add new volume
	CannotAddNewVolume,
	/// Cannot remove volume
	CannotRemoveVolume,
	/// An internal server error occurred. This should not happen unless there
	/// is a bug in the server
	InternalServerError,
	/// A role with the given name already exists
	RoleAlreadyExists,
	/// A role with that ID does not exist
	RoleDoesNotExist,
	/// The API token does not exist
	ApiTokenDoesNotExist,
	/// An API token with the given name already exists
	ApiTokenAlreadyExists,
	/// The role that the user is trying to delete is in use and cannot be
	/// deleted
	RoleInUse,
	/// Another instance of the same runner ID is already connected
	RunnerAlreadyConnected,
	/// The operation is not allowed in the current runner mode
	InvalidRunnerMode,
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
			Self::MfaAlreadyActive => StatusCode::CONFLICT,
			Self::MfaAlreadyInactive => StatusCode::CONFLICT,
			Self::TagNotFound => StatusCode::BAD_REQUEST,
			Self::WrongParameters => StatusCode::BAD_REQUEST,
			Self::MalformedApiToken => StatusCode::BAD_REQUEST,
			Self::DisallowedIpAddressForApiToken => StatusCode::UNAUTHORIZED,
			Self::MalformedAccessToken => StatusCode::BAD_REQUEST,
			Self::MalformedRefreshToken => StatusCode::BAD_REQUEST,
			Self::Unauthorized => StatusCode::UNAUTHORIZED,
			Self::AuthorizationTokenInvalid => StatusCode::UNAUTHORIZED,
			Self::UsernameUnavailable => StatusCode::CONFLICT,
			Self::EmailUnavailable => StatusCode::CONFLICT,
			Self::PhoneUnavailable => StatusCode::CONFLICT,
			Self::InvalidPasswordResetToken => StatusCode::BAD_REQUEST,
			Self::ResourceDoesNotExist => StatusCode::NOT_FOUND,
			Self::ResourceAlreadyExists => StatusCode::CONFLICT,
			Self::ResourceInUse => StatusCode::UNPROCESSABLE_ENTITY,
			Self::WorkspaceNameAlreadyExists => StatusCode::CONFLICT,
			Self::WorkspaceNotEmpty => StatusCode::FAILED_DEPENDENCY,
			Self::CannotReduceVolumeSize => StatusCode::BAD_REQUEST,
			Self::CannotAddNewVolume => StatusCode::BAD_REQUEST,
			Self::CannotRemoveVolume => StatusCode::BAD_REQUEST,
			Self::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
			Self::RoleAlreadyExists => StatusCode::CONFLICT,
			Self::RoleDoesNotExist => StatusCode::NOT_FOUND,
			Self::ApiTokenDoesNotExist => StatusCode::NOT_FOUND,
			Self::ApiTokenAlreadyExists => StatusCode::CONFLICT,
			Self::RoleInUse => StatusCode::CONFLICT,
			Self::RunnerAlreadyConnected => StatusCode::CONFLICT,
			Self::InvalidRunnerMode => StatusCode::FORBIDDEN,
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
			Self::MfaAlreadyActive => {
				"Two factor authentication is already enabled on your account"
			}
			Self::MfaAlreadyInactive => "Two factor authentication is not enabled on your account",
			Self::TagNotFound => "No tag exists",
			Self::WrongParameters => "The parameters sent with that request is invalid",
			Self::MalformedApiToken => "The API token provided is not a valid token",
			Self::DisallowedIpAddressForApiToken => {
				"The API token provided is not allowed from this IP address"
			}
			Self::MalformedAccessToken => "Your access token is invalid. Please login again",
			Self::MalformedRefreshToken => "Your refresh token is invalid. Please login again",
			Self::Unauthorized => "You are not authorized to perform that action",
			Self::AuthorizationTokenInvalid => "Your access token has expired. Please login again",
			Self::UsernameUnavailable => "An account already exists with that username",
			Self::EmailUnavailable => "An account already exists with that email",
			Self::PhoneUnavailable => "An account already exists with that phone number",
			Self::InvalidPasswordResetToken => {
				"The token provided to reset your password is not valid"
			}
			Self::ResourceDoesNotExist => "The resource you are trying to access does not exist",
			Self::ResourceAlreadyExists => "Resource already exists with the given details",
			Self::ResourceInUse => "Resource is currently in use",
			Self::WorkspaceNameAlreadyExists => "A workspace with that name already exists",
			Self::WorkspaceNotEmpty => "A workspace cannot be deleted until all the resources in the workspaces have been deleted",
			Self::CannotReduceVolumeSize => "The deployment volume size cannot be reduced",
			Self::CannotAddNewVolume => "New volume cannot be added",
			Self::CannotRemoveVolume => "The volume cannot be removed",
			Self::InternalServerError => "An internal server error has occured",
			Self::RoleAlreadyExists => "A role with that name already exists",
			Self::RoleDoesNotExist => "A role with that ID does not exist",
			Self::ApiTokenDoesNotExist => "The API token does not exist",
			Self::ApiTokenAlreadyExists => "An API token with that name already exists",
			Self::RoleInUse => "The role is currently assigned to users and cannot be deleted",
			Self::RunnerAlreadyConnected => "Another instance of the same runner ID is already connected",
			Self::InvalidRunnerMode => "That operation is not allowed in the mode the runner is currently in",
		}
	}

	/// Creates an [`ErrorType::InternalServerError`] with the given message
	pub fn server_error(message: impl Display) -> Self {
		error!("Internal server error occured: {message}");
		Self::InternalServerError
	}
}

impl FromStr for ErrorType {
	type Err = ErrorType;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Self::iter()
			.find(|error_type| error_type.to_string() == s)
			.ok_or(ErrorType::InternalServerError)
	}
}

impl<Error> From<Error> for ErrorType
where
	Error: StdError + Send + Sync + 'static,
{
	fn from(error: Error) -> Self {
		error!(
			"Creating error type from error `{}`: {}",
			std::any::type_name::<Error>(),
			error
		);
		Self::InternalServerError
	}
}
