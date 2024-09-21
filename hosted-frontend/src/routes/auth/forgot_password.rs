macros::declare_app_route! {
	/// Route for the page to trigger a password reset
	ForgotPassword,
	"/forgot-password",
	requires_login = false,
	query = {
		/// The userId to prefill the forgot password form with
		#[serde(skip_serializing_if = "Option::is_none")]
		pub user_id: Option<String>,
	},
}
