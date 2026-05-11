/// The endpoint to update the email of a user
mod update_user_email;
/// The endpoint to verify the email of a user
mod verify_user_email;

pub use self::{update_user_email::*, verify_user_email::*};
