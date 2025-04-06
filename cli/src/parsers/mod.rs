/// A parser that prompts the user for a password. This is used for
/// passwords and not normal texts.
mod password_input;
/// A parser that prompts the user for input. This is used for normal texts
/// and not passwords.
mod text_input;

/// The default message to display when the terminal is not interactive.
pub const DEFAULT_NO_TTY_MESSAGE: &str = "Not an interactive terminal. Cannot prompt for input.";

pub use self::{password_input::*, text_input::*};
