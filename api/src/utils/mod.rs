mod eve_context;
mod eve_middlewares;

pub mod constants;
pub mod logger;
#[allow(unused_variables)]
pub mod mailer;
pub mod settings;
pub mod sms;
pub mod validator;

use std::time::{SystemTime, UNIX_EPOCH};

pub use eve_context::*;
pub use eve_middlewares::*;
use eve_rs::Error as EveError;

pub type ErrorData = ();
pub type Error = EveError<ErrorData>;

pub fn get_current_time() -> u64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("Time went backwards. Wtf?")
		.as_millis() as u64
}
