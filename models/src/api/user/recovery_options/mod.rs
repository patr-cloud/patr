/// The endpoint to update the email of a user
mod update_user_email;
/// The endpoint to update the phone number of a user
mod update_user_phone_number;
/// The endpoint to verify the email of a user
mod verify_user_email;
/// The endpoint to verify the phone number of a user
mod verify_user_phone_number;

pub use self::{
	update_user_email::*,
	update_user_phone_number::*,
	verify_user_email::*,
	verify_user_phone_number::*,
};
