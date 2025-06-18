use crate::prelude::*;

/// The Login Page
mod login;

pub use self::login::*;

/// The list of all pages in the application that does not require the user to
/// be logged in
#[derive(Debug, Clone, PartialEq, Eq, Routable)]
pub enum LoggedOutRoutes {
	/// The Login page. This is the page that is displayed when the user
	/// tries to access a page that requires authentication but is not logged
	/// in.
	#[route(
		"/login?:to&:username_error&:password_error&:show_mfa_input",
		LoginPage
	)]
	Login {
		/// The `to` query parameter is used to redirect the user to the page
		/// they were trying to access before they were redirected to the login
		/// page. This is useful for redirecting the user back to the page they
		/// were trying to access after they log in.
		/// If the `to` query parameter is not present, the user will be
		/// redirected to the home page after logging in.
		to: String,
		/// If the user has an error while logging in, the `username_error`
		/// field will be set to the error message for the username field.
		username_error: String,
		/// If the user has an error while logging in, the `password_error`
		/// field will be set to the error message for the password field.
		password_error: String,
		/// If the user tries to log in with an invalid MFA OTP, the
		/// `show_mfa_input` field will be set to `true`, and the user will be
		/// prompted to enter their MFA OTP.
		show_mfa_input: bool,
	},
	/// The Sign Up page. This is the page that is displayed when the user
	/// wants to create a new account.
	#[route("/sign-up", NotFoundPage)]
	SignUp,
	/// The Forgot Password page. This is the page that is displayed when the
	/// user wants to trigger a password reset flow.
	#[route("/forgot-password", NotFoundPage)]
	ForgotPassword,
	/// The Reset Password page. This is the page that is displayed when the
	/// user wants to reset their password after triggering a password reset
	/// flow. This is where the user will enter their new password after
	/// receiving a password reset link via email.
	#[route("/reset-password", NotFoundPage)]
	ResetPassword,
}
