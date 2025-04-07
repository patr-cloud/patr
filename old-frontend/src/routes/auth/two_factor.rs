macros::declare_app_route! {
	/// Route for the 2FA input page
	TwoFactor,
	"/mfa",
	requires_login = false,
	query = {

	},
}
