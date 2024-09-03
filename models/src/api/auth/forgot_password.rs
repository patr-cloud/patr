use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// Recovery method options provided to the user.
///
/// When they forget their password and request a password change by hitting the
/// ForgetPassword API endpoint, these are the options presented to them. The
/// current recovery options are email and phone number.
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
	api = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The user identifier. It can either be a username or an email ID
		/// depending on what user enters
		#[preprocess(trim, length(min = 2))]
		pub user_id: String,
		/// Recovery method the user wants to use to reset his password
		#[preprocess(none)]
		pub preferred_recovery_option: PreferredRecoveryOption,
	},
);
