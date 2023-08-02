mod add_email_address; // Add personal email address
mod add_phone_number; // Add phone number
mod delete_personal_email;
mod delete_phone_number;
mod list_email_address;
mod list_phone_numbers;
mod update_backup_email;
mod update_backup_phone_number;
mod verify_email_address;
mod verify_phone_number;

pub use self::{
	add_email_address::*,
	add_phone_number::*,
	delete_personal_email::*,
	delete_phone_number::*,
	list_email_address::*,
	list_phone_numbers::*,
	update_backup_email::*,
	update_backup_phone_number::*,
	verify_email_address::*,
	verify_phone_number::*,
};
