#![feature(exitcode_exit_method)]

//! A CLI tool for interacting and managing your Patr resources.

use std::process::ExitCode;

use clap::Parser;

use crate::prelude::*;

/// All items related to running the CLI goes here
mod app;
/// All the commands, arguments and the functionalities for it.
mod commands;
/// Utilities module for helper functions, structs, and enums.
mod utils;

/// A prelude that re-exports commonly used items.
pub mod prelude {
	pub use models::prelude::*;
	pub use tracing::{debug, error, info, instrument, trace, warn};

	pub use crate::{
		app::{CommandExecutor, CommandOutput, OutputType},
		commands::{AppArgs, GlobalArgs, GlobalCommands},
		utils::{constants, make_request, AppState, ToJsonValue},
	};
}

#[tokio::main]
async fn main() -> ExitCode {
	let AppArgs {
		global_args,
		command,
	} = AppArgs::parse();

	let state = AppState::load()
		.inspect_err(|err| {
			eprintln!("Failed to load the CLI state: {}", err);
			eprintln!("Loading default state...");
		})
		.unwrap_or_default();

	let output_format = global_args.output;
	let Ok(output) = command.execute(global_args, state).await.map_err(|err| {
		eprintln!(
			"{}",
			match output_format {
				OutputType::Text => {
					err.body.message
				}
				OutputType::Json => {
					serde_json::to_string(&err.body).unwrap()
				}
				OutputType::PrettyJson => {
					serde_json::to_string_pretty(&err.body).unwrap()
				}
			}
		)
	}) else {
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
