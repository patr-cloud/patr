use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorType {
	InvalidPassword,
	Unauthorized,
	InvalidEmail,
	Expired,
	NotFound,
	UserNotFound,
	TokenNotFound,
	EmailTokenNotFound,
	PasswordTooWeak,
	PasswordUnchanged,
	WrongParameters,
	InvalidBirthday,
	InvalidStateValue,
	InvalidPhoneNumber,
	ResourceInUse,
	ResourceExists,
	CannotDeleteWorkspace,
	InvalidWorkspaceName,
	InvalidRepositoryName,
	PaymentFailed,
	FeatureNotSupportedForCustomCluster,
}
