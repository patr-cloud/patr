/// The error type for Iaac operations. This is used to handle errors that
/// occur while parsing Iaac files, validating resources, and other Iaac-related
/// operations.
#[derive(Debug, thiserror::Error)]
pub enum IaacError {
	/// An error occurred while parsing an environment variable value.
	#[error("error parsing environment variable value: {0}")]
	EnvironmentVariableParseError(String),
	/// An error that occurred while trying to resolve an environment variable.
	#[error("environment variable not found: {0}")]
	EnvironmentVariableNotFound(String),
	/// A duplicate resource was found in the Iaac file.
	#[error("duplicate resource found: {0}")]
	DuplicateResource(String),
	/// A resource was not found in the Iaac file.
	#[error("resource not found: {0}")]
	ResourceNotFound(String),
}
