/// The forgot password route
mod forgot_password;
/// The login route
mod login;
/// The reset password route
mod reset_password;
/// The sign-up route
mod sign_up;
/// The 2FA input route
mod two_factor;
/// The sign up verification route
mod verify_signup;

pub use self::{
	forgot_password::*,
	login::*,
	reset_password::*,
	sign_up::*,
	two_factor::*,
	verify_signup::*,
};
