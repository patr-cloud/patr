#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! A CLI tool for interacting and managing your Patr resources.

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
	pub use tracing::{debug, error, info, instrument, trace, warn};

	pub use crate::{
		app::{CommandExecutor, CommandOutput, OutputType},
		commands::{AppArgs, GlobalArgs, GlobalCommands},
		utils::{constants, make_request, AppState},
	};
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let AppArgs {
		global_args,
		command,
	} = AppArgs::parse();

	let output = command.execute(&global_args).await?;

	println!(
		"{}",
		match global_args.output {
			OutputType::Text => {
				output.text
			}
			OutputType::Json => {
				serde_json::to_string(&output.json).unwrap()
			}
			OutputType::PrettyJson => {
				serde_json::to_string_pretty(&output.json).unwrap()
			}
		}
	);

	Ok(())
}
