macros::declare_app_route! {
	/// Route for the login page
	SignUp,
	"/sign-up",
	requires_login = false,
	query = {
		/// The next page to redirect to after signing up
		#[serde(skip_serializing_if = "Option::is_none")]
		pub next: Option<String>,
		/// The userId to prefill the sign-up form with
		#[serde(skip_serializing_if = "Option::is_none")]
		pub username: Option<String>,
		/// The email to prefill the sign-up form with
		#[serde(skip_serializing_if = "Option::is_none")]
		pub email: Option<String>,
		/// The first name to prefill the sign-up form with
		#[serde(skip_serializing_if = "Option::is_none")]
		pub first_name: Option<String>,
		/// The last name to prefill the sign-up form with
		#[serde(skip_serializing_if = "Option::is_none")]
		pub last_name: Option<String>,
	},
}
