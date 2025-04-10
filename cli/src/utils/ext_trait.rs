use std::fmt::Display;

use inquire::InquireError;

/// Trait to extend the `Result` type with a method to handle TTY expectations.
/// This trait provides a method to handle the case where the terminal is not a
/// TTY when expecting user input.
pub trait TtyExpectable<T> {
	/// Handles the case where the terminal is not a TTY when expecting user
	/// input. If the result is `Ok`, it returns the value. If the result is an
	/// error indicating that the terminal is not a TTY, it prints an error
	/// message and exits the process with a failure code.
	fn expect_tty(self, message: impl Display) -> T;
}

impl<T> TtyExpectable<T> for Result<T, InquireError> {
	fn expect_tty(self, message: impl Display) -> T {
		let message = message.to_string();
		match self {
			Ok(value) => value,
			Err(InquireError::NotTTY) => {
				eprintln!(concat!(
					"The terminal the CLI is running in is not a TTY. ",
					"You either need to provide a CLI flag for the value you are trying to set, ",
					"or use an interactive terminal to allow the CLI to prompt you for the value.",
				));
				std::process::ExitCode::FAILURE.exit_process();
			}
			err => err.expect(&message),
		}
	}
}
