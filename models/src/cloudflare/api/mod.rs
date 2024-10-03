use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// The APIs related to Cloudflare Tunnels
pub mod tunnel;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ApiResponse<T> {
	Success {
		success: True,
		result: T,
	},
	Error {
		success: False,
		errors: Vec<ErrorMessage>,
		messages: Vec<ErrorMessage>,
	},
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorMessage {
	code: usize,
	message: String,
}
