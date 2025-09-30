/// The Forgot Password Page
mod forgot_password;
/// The Login Page
mod login;
/// The Sign Up Page
mod sign_up;
/// The Confirm Sign Up Page. Once a sign up is done, the user will be
/// redirected here to confirm their account with an OTP sent to their email
mod verify_sign_up;

pub use self::{forgot_password::*, login::*, sign_up::*, verify_sign_up::*};
