macros::declare_app_route! {
	/// Route for verifying a signup with a token
	VerifySignUp,
	"/confirm",
	requires_login = false,
	query = {
		/// The userId to prefill the forgot password form with
		#[serde(skip_serializing_if = "Option::is_none")]
		pub user_id: Option<String>,
		/// The signup token to prefill the forgot password form with
		#[serde(skip_serializing_if = "Option::is_none")]
		pub signup_token: Option<String>,
	},
}
