use config::ConfigError;
use headers::Error as HeaderError;
use models::{ApiErrorResponse, ErrorType};
use reqwest::Error as ReqwestError;

/// The error type for the CLI application. This is used to handle errors that
/// occur while executing commands, making requests to the API, and parsing
/// data.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
	/// The user is not logged in.
	#[error("user is not logged in")]
	NotLoggedIn,
	/// An upstream error that occurred while making a request to the API.
	#[error("API error: {0}")]
	ApiError(ErrorType),
	/// An error that occurred while parsing internal data.
	#[error("error parsing data: {0}")]
	ParseError(String),
	/// An error that occurred while making a request to the API.
	#[error("error making HTTP request: {0}")]
	NetworkError(#[from] ReqwestError),
	/// An error that occurred while trying to load the CLI state.
	#[error("error loading CLI state: {0}")]
	ConfigReadError(ConfigError),
	/// An error that occurred while trying to save the CLI state.
	#[error("error saving CLI state: {0}")]
	ConfigWriteError(ConfigError),
}

impl From<ApiErrorResponse> for AppError {
	fn from(error: ApiErrorResponse) -> Self {
		Self::ApiError(error.body.error)
	}
}

impl From<HeaderError> for AppError {
	fn from(error: HeaderError) -> Self {
		Self::ParseError(error.to_string())
	}
}
