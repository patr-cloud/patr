use eve_rs::{AsError, Error as EveError};
use serde_json::json;

use crate::utils::constants::request_keys::{ERROR, MESSAGE, SUCCESS};

pub trait AsErrorResponse {
	fn get_status_code(&self) -> u16;

	fn get_message(&self) -> Option<&str>;

	fn get_id(&self) -> Option<&str>;
}

pub trait EnumError<V, U, T> {
	fn with_error(self, err: T) -> Result<V, U>;
}

impl<Value, TErrorData, T> EnumError<Value, EveError<TErrorData>, T>
	for Result<Value, EveError<TErrorData>>
where
	T: AsErrorResponse,
	Value: Send + Sync,
	TErrorData: Default + Send + Sync,
{
	fn with_error(self, err: T) -> Result<Value, EveError<TErrorData>> {
		if err.get_message().is_some() && err.get_id().is_some() {
			let msg =
				err.get_message().expect("Message was some, yet it is none");
			let id = err.get_id().expect("Id was some, yet it is none");
			self.status(err.get_status_code()).body(
				json!({
					SUCCESS: false,
					MESSAGE: msg,
					ERROR: id
				})
				.to_string(),
			)
		} else {
			self.status(err.get_status_code())
		}
	}
}

impl<Value, TErrorData, T> EnumError<Value, EveError<TErrorData>, T>
	for Option<Value>
where
	T: AsErrorResponse,
	Value: Send + Sync,
	TErrorData: Default + Send + Sync,
{
	fn with_error(self, err: T) -> Result<Value, EveError<TErrorData>> {
		if err.get_message().is_some() && err.get_id().is_some() {
			let msg =
				err.get_message().expect("Message was some, yet it is none");
			let id = err.get_id().expect("Id was some, yet it is none");
			return self.status(err.get_status_code()).body(
				json!({
					SUCCESS: false,
					MESSAGE: msg,
					ERROR: id
				})
				.to_string(),
			);
		} else {
			self.status(err.get_status_code())
		}
	}
}
