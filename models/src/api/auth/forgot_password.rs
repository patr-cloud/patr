use serde::{Deserialize, Serialize};

/// Recovery method options provided to the user when they forget their
/// passsword and request a password change by hitting the ForgetPassword API
/// endpoint. The curent recovery options are email and phone number.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PreferredRecoveryOption {
	/// Send OTP to phone number
	RecoveryPhoneNumber,
	/// Send OTP to email address
	RecoveryEmail,
}

macros::declare_api_endpoint!(
	/// Route when user forgets their password and raises a password change request.
	/// This will send an OTP to the selected recovery method.
	ForgotPassword,
	POST "/auth/forgot-password",
	request = {
		/// The user identifier. It can either be a username or an email ID
		/// depending on what user enters
		pub user_id: String,
		/// Recovery method the user wants to use to reset his password
		pub preferred_recovery_option: PreferredRecoveryOption,
	},
);
