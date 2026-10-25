use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to resent an OTP to the linked recovery method opted by the user to
	/// verify their account. The recovery method can either be an email or a phone number.
	ResendOtp,
	POST "/auth/resend-otp",
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The email address of the user
		#[preprocess(trim, email)]
		pub email: String,
	},
	client_type = [WebLogin],
	audit_log = NoAuditLogger,
);
