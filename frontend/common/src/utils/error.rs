use serde::{Deserialize, Serialize};
use server_fn::codec::JsonEncoding;
use strum::Display;

use crate::prelude::*;
#[derive(Debug, Clone, Deserialize, Serialize, Display)]
/// Custom AppError Type taken from [leptos book](https://book.leptos.dev/server/25_server_functions.html?highlight=AppError#using-custom-errors)
pub enum AppError {
	/// An error that occurred while calling a server function.
	ServerFnError(ServerFnErrorErr),
	/// A general error with a message.
	General(String),
}

impl FromServerFnError for AppError {
	type Encoder = JsonEncoding;

	fn from_server_fn_error(value: ServerFnErrorErr) -> Self {
		AppError::ServerFnError(value)
	}
}
