#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::missing_docs_in_private_items)]
#![cfg_attr(debug_assertions, allow(unused_variables, dead_code))]

//! A CLI tool for interacting and managing your Patr resources.

use clap::Parser;
use commands::{AppArgs, CommandExecutor};

/// All the comamnds, arguments and the functionalities for it.
mod commands;
/// All the structs for the JSON output as well as the tabled output.
mod models;
/// Utilities module for helper functions, structs, and enums.
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let args = AppArgs::parse();

	args.command
		.execute(&args.global_args, (), std::io::stdout())
		.await?;

	Ok(())
}
