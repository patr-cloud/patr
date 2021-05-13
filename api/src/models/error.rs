#[allow(dead_code)]
pub mod id {
	pub const USER_NOT_FOUND: &str = "userNotFound";
	pub const EMAIL_NOT_VERIFIED: &str = "emailNotVerified";
	pub const INVALID_PASSWORD: &str = "invalidPassword";
	pub const INVALID_EMAIL: &str = "invalidEmail";
	pub const INVALID_CREDENTIALS: &str = "invalidCredentials";
	pub const INVALID_USERNAME: &str = "invalidUsername";
	pub const INVALID_PHONE_NUMBER: &str = "invalidPhoneNumber";
	pub const INVALID_COUNTRY_CODE: &str = "invalidCountryCode";
	pub const INVALID_ORGANISATION_NAME: &str = "invalidOrganisationName";
	pub const ORGANISATION_EXISTS: &str = "organisationExists";
	pub const PASSWORD_TOO_WEAK: &str = "passwordTooWeak";
	pub const WRONG_PARAMETERS: &str = "wrongParameters";
	pub const UNAUTHORIZED: &str = "unauthorized";
	pub const EXPIRED: &str = "expired";
	pub const INVALID_DOMAIN_NAME: &str = "invalidDomainName";
	pub const UNPRIVILEGED: &str = "unprivileged";
	pub const SERVER_ERROR: &str = "serverError";
	pub const EMAIL_TAKEN: &str = "emailTaken";
	pub const USERNAME_TAKEN: &str = "usernameTaken";
	pub const PHONE_NUMBER_TAKEN: &str = "phoneNumberTaken";
	pub const EMAIL_TOKEN_NOT_FOUND: &str = "emailTokenNotFound";
	pub const EMAIL_TOKEN_EXPIRED: &str = "emailTokenExpired";
	pub const INVALID_OTP: &str = "invalidOtp";
	pub const OTP_EXPIRED: &str = "otpExpired";
	pub const NOT_FOUND: &str = "notFound";
	pub const RESOURCE_EXISTS: &str = "resourceExists";
	pub const RESOURCE_DOES_NOT_EXIST: &str = "resourceDoesNotExist";
	pub const PROFILE_NOT_FOUND: &str = "profileNotFound";
	pub const DUPLICATE_USER: &str = "duplicateUser";
	pub const DOMAIN_UNVERIFIED: &str = "domainUnverified";
	pub const DOMAIN_IS_PERSONAL: &str = "domainIsPersonal";
	pub const DOMAIN_BELONGS_TO_ORGANISATION: &str =
		"domainBelongsToOrganisation";
	pub const NO_RECOVERY_OPTIONS: &str = "noRecoveryOptions";
}

#[allow(dead_code)]
pub mod message {
	pub const USER_NOT_FOUND: &str = "The document you are looking for is either deleted or has been moved. Please check your link again";
	pub const EMAIL_NOT_VERIFIED: &str = "Your email address is not verified";
	pub const INVALID_PASSWORD: &str = "Your password is incorrect";
	pub const INVALID_EMAIL: &str = "Your email address is invalid";
	pub const INVALID_CREDENTIALS: &str = "Your credentials are not valid";
	pub const INVALID_USERNAME: &str = "Your username is not valid";
	pub const INVALID_PHONE_NUMBER: &str =
		"Your phone number seems to be incorrect";
	pub const INVALID_COUNTRY_CODE: &str =
		"Your country code seems to be incorrect";
	pub const INVALID_ORGANISATION_NAME: &str =
		"That organisation name is not valid";
	pub const ORGANISATION_EXISTS: &str =
		"That organisation name is already taken";
	pub const PASSWORD_TOO_WEAK: &str =
		"Your password is too weak. Please choose a stronger password";
	pub const WRONG_PARAMETERS: &str =
		"An internal error occured. This incident has been reported";
	pub const UNAUTHORIZED: &str =
		"An error occured. If this persists, please contact the administrator";
	pub const EXPIRED: &str =
		"An error occured. If this persists, please try logging in again";
	pub const INVALID_DOMAIN_NAME: &str =
		"That doesn't seem to be a valid domain name. Please try another name";
	pub const UNPRIVILEGED: &str =
		"You do not have the permission to perform that action";
	pub const SERVER_ERROR: &str =
		"An internal server error has occured. Please try again later";
	pub const EMAIL_TAKEN: &str =
		"Sorry. That email address is already in use.";
	pub const USERNAME_TAKEN: &str = "Sorry. That username is taken.";
	pub const PHONE_NUMBER_TAKEN: &str =
		"That phone number is already in use. Did you mean to sign in?";
	pub const EMAIL_TOKEN_NOT_FOUND: &str =
		"Your link seems to be invalid. Please request for a new link again";
	pub const EMAIL_TOKEN_EXPIRED: &str =
		"Your link has expired. Please request for a new link again";
	pub const INVALID_OTP: &str = "That OTP seems to be invalid";
	pub const OTP_EXPIRED: &str =
		"That OTP seems to have been expired. Please request a new one";
	pub const NOT_FOUND: &str = "That route doesn't seem to exist";
	pub const RESOURCE_EXISTS: &str = "That resource already exists";
	pub const RESOURCE_DOES_NOT_EXIST: &str =
		"That resource doesn't seem to exist";
	pub const PROFILE_NOT_FOUND: &str = "The profile doesn't seem to exist";
	pub const DUPLICATE_USER: &str =
		"Sorry, the email address/username is taken";
	pub const DOMAIN_UNVERIFIED: &str =
		"That domain is unverified. Check your verification settings";
	pub const DOMAIN_IS_PERSONAL: &str =
		"That domain seems to be used for a personal account. Please remove all personal accounts related to that domain first. If this problem persists, please contact us";
	pub const DOMAIN_BELONGS_TO_ORGANISATION: &str =
		"That domain seems to belong to an organisation. Please choose a personal domain instead. If this problem persists, please contact us";
	pub const NO_RECOVERY_OPTIONS: &str =
		"You seem to have no recovery options set for your account. Please add either a backup email or a backup phone number";
}
