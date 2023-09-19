use serde::{Deserialize, Serialize};

/// Recovery method options provided to the user
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PreferredRecoveryOption {
	/// Send OTP to phone number
	RecoveryPhoneNumber,
	/// Send OTP to email address
	RecoveryEmail,
}

macros::declare_api_endpoint!(
	/// Definition of a route when user forgets their password and raises a password change request.
	/// This will send an OTP to the selected recovery method.
	ForgotPassword,
	POST "/auth/forgot-password",
	request = {
		/// The user ID of the user
		pub user_id: String,
		/// Recovery method the user wants to use to reset his password
		pub preferred_recovery_option: PreferredRecoveryOption,
	},
);
