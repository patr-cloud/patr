#![feature(exitcode_exit_method)]

//! A CLI tool for interacting and managing your Patr resources.

/// All items related to running the CLI goes here
mod app;
/// All the commands, arguments and the functionalities for it.
mod commands;
/// The errors thrown by the CLI.
mod error;
/// Parsers for the CLI arguments. This module contains all the parsers
/// for the CLI arguments and subcommands that can be used to interface with
/// [`inquire`].
mod parsers;
/// Utilities module for helper functions, structs, and enums.
mod utils;

/// A prelude that re-exports commonly used items.
pub mod prelude {
	pub use models::prelude::*;
	pub use tracing::{debug, error, info, instrument, trace, warn};

	pub use crate::{
		app::{CommandOutput, OutputType},
		commands::{AppArgs, GlobalArgs, GlobalCommand},
		error::AppError,
		parsers::*,
		utils::{AppState, ToJsonValue, constants, make_request},
	};
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
	use std::process::ExitCode;

	use clap::Parser;
	use models::{ApiErrorResponseBody, utils::False};

	use crate::prelude::*;

	let AppArgs { args, command } = AppArgs::parse();

	let state = AppState::load()
		.inspect_err(|err| {
			eprintln!("{}", err);
			eprintln!("Loading default state...");
		})
		.unwrap_or_default();

	let output_format = args.output;
	let Ok(output) = commands::execute(command, args, state)
		.await
		.map_err(|err| {
			let error_response = ApiErrorResponseBody {
				success: False,
				error: match &err {
					AppError::ApiError(err) => *err,
					other => ErrorType::server_error(other),
				},
				message: err.to_string(),
			};
			eprintln!(
				"{}",
				match output_format {
					OutputType::Text => {
						err.to_string()
					}
					OutputType::Json => {
						serde_json::to_string(&error_response).unwrap()
					}
					OutputType::PrettyJson => {
						serde_json::to_string_pretty(&error_response).unwrap()
					}
				}
			)
		})
	else {
		return ExitCode::FAILURE;
	};

	match output_format {
		OutputType::Text => {
			eprintln!("{}", output.text)
		}
		OutputType::Json => {
			println!("{}", serde_json::to_string(&output.json).unwrap());
		}
		OutputType::PrettyJson => {
			println!("{}", serde_json::to_string_pretty(&output.json).unwrap());
		}
	}

	ExitCode::SUCCESS
}
