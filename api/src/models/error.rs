#[allow(dead_code)]
pub mod id {
	pub const USER_NOT_FOUND: &str = "userNotFound";
	pub const EMAIL_NOT_VERIFIED: &str = "emailNotVerified";
	pub const EMAIL_NOT_FOUND: &str = "emailNotFound";
	pub const PHONE_NUMBER_NOT_FOUND: &str = "phoneNumberNotFound";
	pub const INVALID_PASSWORD: &str = "invalidPassword";
	pub const INVALID_EMAIL: &str = "invalidEmail";
	pub const INVALID_CREDENTIALS: &str = "invalidCredentials";
	pub const INVALID_USERNAME: &str = "invalidUsername";
	pub const INVALID_PHONE_NUMBER: &str = "invalidPhoneNumber";
	pub const INVALID_COUNTRY_CODE: &str = "invalidCountryCode";
	pub const INVALID_WORKSPACE_NAME: &str = "invalidWorkspaceName";
	pub const WORKSPACE_EXISTS: &str = "workspaceExists";
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
	pub const TOKEN_NOT_FOUND: &str = "tokenNotFound";
	pub const EMAIL_TOKEN_NOT_FOUND: &str = "emailTokenNotFound";
	pub const EMAIL_TOKEN_EXPIRED: &str = "emailTokenExpired";
	pub const PHONE_NUMBER_TOKEN_NOT_FOUND: &str = "phoneTokenNotFound";
	pub const PHONE_NUMBER_TOKEN_EXPIRED: &str = "phoneTokenNotFound";
	pub const INVALID_OTP: &str = "invalidOtp";
	pub const OTP_EXPIRED: &str = "otpExpired";
	pub const NOT_FOUND: &str = "notFound";
	pub const RESOURCE_EXISTS: &str = "resourceExists";
	pub const RESOURCE_DOES_NOT_EXIST: &str = "resourceDoesNotExist";
	pub const PROFILE_NOT_FOUND: &str = "profileNotFound";
	pub const DUPLICATE_USER: &str = "duplicateUser";
	pub const DOMAIN_UNVERIFIED: &str = "domainUnverified";
	pub const REPOSITORY_ALREADY_EXISTS: &str = "repositoryAlreadyExists";
	pub const REPOSITORY_NOT_FOUND: &str = "repositoryNotFound";
	pub const INVALID_REQUEST: &str = "invalidRequest";
	pub const INVALID_REPOSITORY_NAME: &str = "invalidRepositoryName";
	pub const DOMAIN_IS_PERSONAL: &str = "domainIsPersonal";
	pub const DOMAIN_BELONGS_TO_WORKSPACE: &str = "domainBelongsToWorkspace";
	pub const NO_RECOVERY_OPTIONS: &str = "noRecoveryOptions";
	pub const CANNOT_DELETE_RECOVERY_EMAIL: &str = "cannotDeleteRecoveryEmail";
	pub const CANNOT_DELETE_RECOVERY_PHONE_NUMBER: &str =
		"cannotDeleteRecoveryPhoneNumber";
	pub const DOMAIN_EXISTS: &str = "domainExists";
	pub const INVALID_DEPLOYMENT_NAME: &str = "invalidDeploymentName";
	pub const INVALID_STATIC_SITE_NAME: &str = "invalidStaticSiteName";
	pub const RESOURCE_IN_USE: &str = "resourceInUse";
	pub const DOMAIN_NOT_PATR_CONTROLLED: &str = "domainNotPatrControlled";
	pub const INVALID_IP_ADDRESS: &str = "invalidIpAddress";
	pub const DNS_RECORD_NOT_FOUND: &str = "dnsRecordNotFound";
	pub const INVALID_DNS_RECORD_NAME: &str = "invalidDnsRecordName";
	pub const MAX_LIMIT_REACHED: &str = "maxLimitReached";
	pub const CANNOT_DELETE_WORKSPACE: &str = "cannotDeleteWorkspace";
	pub const ADDRESS_LINE_3_NOT_ALLOWED: &str = "addressLine3NotAllowed";
	pub const CHANGE_PRIMARY_PAYMENT_METHOD: &str =
		"changePrimaryPaymentMethod";
	pub const CANNOT_DELETE_PAYMENT_METHOD: &str = "cannotDeletePaymentMethod";
	pub const INVALID_COUPON: &str = "couponDoesNotExist";
	pub const COUPON_USED: &str = "couponUsed";
	pub const PRODUCT_LIMIT_CROSSED: &str = "productLimitCrossed";
	pub const RESOURCE_LIMIT_REACHED: &str = "resourceLimitReached";
	pub const ADDRESS_NOT_FOUND: &str = "addressNotFound";
}

#[allow(dead_code)]
pub mod message {
	pub const USER_NOT_FOUND: &str = "The document you are looking for is either deleted or has been moved. Please check your link again";
	pub const EMAIL_NOT_VERIFIED: &str = "Your email address is not verified";
	pub const EMAIL_NOT_FOUND: &str = "The email address sent by the client could not be found in the database.";
	pub const PHONE_NUMBER_NOT_FOUND: &str = "The phone number is not found";
	pub const INVALID_PASSWORD: &str = "Your password is incorrect";
	pub const INVALID_EMAIL: &str = "Your email address is invalid";
	pub const INVALID_CREDENTIALS: &str = "Your credentials are not valid";
	pub const INVALID_USERNAME: &str = "Your username is not valid";
	pub const INVALID_PHONE_NUMBER: &str =
		"Your phone number seems to be incorrect";
	pub const INVALID_COUNTRY_CODE: &str =
		"Your country code seems to be incorrect";
	pub const INVALID_WORKSPACE_NAME: &str = "That workspace name is not valid";
	pub const WORKSPACE_EXISTS: &str = "That workspace name is already taken";
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
	pub const LOGIN_FAILURE: &str =
		"An error occured during logging into the registry please check your credentials";
	pub const EMAIL_TAKEN: &str = "Sorry. That email address is already in use";
	pub const USERNAME_TAKEN: &str = "Sorry. That username is taken";
	pub const PHONE_NUMBER_TAKEN: &str =
		"That phone number is already in use. Did you mean to sign in?";
	pub const TOKEN_NOT_FOUND: &str =
		"Your account has been logged out due to inactivity. Please login again";
	pub const EMAIL_TOKEN_NOT_FOUND: &str =
		"Your link seems to be invalid. Please request for a new link again";
	pub const EMAIL_TOKEN_EXPIRED: &str =
		"Your link has expired. Please request for a new link again";
	pub const PHONE_NUMBER_TOKEN_NOT_FOUND: &str =
		"Your otp seems to be invalid. Please request for a new otp again";
	pub const PHONE_NUMBER_TOKEN_EXPIRED: &str =
		"Your otp has expired. Please request for a new otp again";
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
	pub const DOMAIN_UNVERIFIED: &str = r#"That domain is unverified. Check your verification settings. 
		Or you might have to wait for the TTL to expire before you can verify it again. 
		Note the TTL is usually set to 3600 seconds"#;
	pub const REPOSITORY_ALREADY_EXISTS: &str =
		"The given repository already exists";
	pub const REPOSITORY_NOT_FOUND: &str = "The repository does not exist";
	pub const ACCESS_TYPE_NOT_PRESENT: &str =
		"Access type not present in request";
	pub const INVALID_ACCESS_TYPE: &str = "Invalid access type sent by client";
	pub const REPOSITORY_NOT_PRESENT: &str =
		"Repository name not present in request";
	pub const ACTION_NOT_PRESENT: &str = "Action not present in request";
	pub const NO_WORKSPACE_OR_REPOSITORY: &str =
		"Invalid Workspace or Repository name";
	pub const INVALID_REPOSITORY_NAME: &str = "Invalid repository name";
	pub const USER_ROLE_NOT_FOUND: &str =
		"No valid role for the user was found";
	pub const OFFLINE_TOKEN_NOT_FOUND: &str =
		"Invalid request sent by the client. Could not find offline_token";
	pub const INVALID_OFFLINE_TOKEN: &str =
		"Invalid request sent by the client. offline_token is not a boolean";
	pub const INVALID_CLIENT_ID: &str =
		"Invalid request sent by the client. Could not find client_id";
	pub const SERVICE_NOT_FOUND: &str =
		"Invalid request sent by the client. Could not find service";
	pub const INVALID_SERVICE: &str =
		"Invalid request sent by the client. Service is not valid";
	pub const AUTHORIZATION_NOT_FOUND: &str =
		"Invalid request sent by the client. Authorization header not found";
	pub const AUTHORIZATION_PARSE_ERROR: &str = "Invalid request sent by the client. Authorization data could not be parsed as expected";
	pub const USERNAME_NOT_FOUND: &str = "Invalid request sent by the client. Authorization header did not have username";
	pub const PASSWORD_NOT_FOUND: &str = "Invalid request sent by the client. Authorization header did not have password";
	pub const DOMAIN_IS_PERSONAL: &str =
		"That domain seems to be used for a personal account. Please remove all personal accounts related to that domain first. If this problem persists, please contact us";
	pub const DOMAIN_BELONGS_TO_WORKSPACE: &str =
		"That domain seems to belong to an workspace. Please choose a personal domain instead. If this problem persists, please contact us";
	pub const NO_RECOVERY_OPTIONS: &str =
		"You seem to have no recovery options set for your account. Please add either a backup email or a backup phone number";
	pub const CANNOT_DELETE_RECOVERY_EMAIL: &str = "The email address sent by the client cannot be deleted because it is assigned as a recovery email. Please update the recovery email first.";
	pub const CANNOT_DELETE_RECOVERY_PHONE_NUMBER: &str = "The phone number sent by the client cannot be deleted because it is assigned as a recovery phone number. Please update the recovery phone number first.";
	pub const DOMAIN_EXISTS: &str = "That domain name is already taken.";
	pub const INVALID_DEPLOYMENT_NAME: &str =
		"Deployment can only consist of alphanumeric characters, spaces, dots, dashes and underscores, and cannot begin or end with a space";
	pub const INVALID_STATIC_SITE_NAME: &str =
		"Static site can only consist of alphanumeric characters, spaces, dots, dashes and underscores, and cannot begin or end with a space";
	pub const RESOURCE_IN_USE: &str = "The resource is currently in use, please delete all the resources connected to it and try again";
	pub const DOMAIN_NOT_PATR_CONTROLLED: &str =
		"The domain has nameservers outside of Patr";
	pub const INVALID_IP_ADDRESS: &str = "The IP address is invalid";
	pub const DNS_RECORD_NOT_FOUND: &str = "The DNS record does not exist";
	pub const INVALID_DNS_RECORD_NAME: &str = "The DNS record name is invalid";
	pub const MAX_LIMIT_REACHED: &str = "You have reached the limit of the maximum number resources allowed for your workspace";
	pub const CANNOT_DELETE_WORKSPACE: &str =
		"You have some resources present in the workspace. Please delete them before proceeding to delete your workspace";
	pub const ADDRESS_LINE_3_NOT_ALLOWED: &str =
		"Address line 3 is not allowed if address line 2 is not provided";
	pub const CHANGE_PRIMARY_PAYMENT_METHOD: &str =
		"The current payment method cannot be deleted since it is your primary payment method";

	pub const CANNOT_DELETE_PAYMENT_METHOD: &str =
		"The payment method cannot be deleted since it is in use, please delete all the resources and try again in the next billing cycle";
	pub const INVALID_COUPON: &str = "The coupon you entered is invalid, please try again with a valid coupon";
	pub const COUPON_USED: &str =
		"The coupon you entered has already been used";
	pub const PRODUCT_LIMIT_CROSSED: &str =
		"You have reached the limit of the maximum number of product allowed for your workspace";
	pub const RESOURCE_LIMIT_REACHED: &str =
		"You have reached the limit of the maximum number resources allowed for your workspace";
	pub const ADDRESS_NOT_FOUND: &str = "The address does not exist";
}
