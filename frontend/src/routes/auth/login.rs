macros::declare_app_route! {
	/// Route for the login page
	Login,
	"/login",
	requires_login = false,
	query = {
		/// The next page to redirect to after login
		#[serde(skip_serializing_if = "Option::is_none")]
		pub next: Option<String>,
		/// The userId to prefill the login form with
		#[serde(skip_serializing_if = "Option::is_none")]
		pub user_id: Option<String>,
	},
}
