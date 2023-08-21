use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PreferredRecoveryOption {
	RecoveryPhoneNumber,
	RecoveryEmail,
}

macros::declare_api_endpoint!(
	ForgotPassword,
	POST "/auth/forgot-password",
	request = {
		pub user_id: String,
		pub preferred_recovery_option: PreferredRecoveryOption,
	},
);
