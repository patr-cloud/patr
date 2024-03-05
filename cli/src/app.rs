use std::{
	fmt::{self, Display, Formatter},
	future::Future,
};

use clap::ValueEnum;
use serde_json::Value;

use crate::prelude::*;

/// The output of a given command
#[derive(Debug, Clone)]
pub struct CommandOutput {
	/// The table / text version of a command's output
	pub text: String,
	/// The JSON output of a command
	pub json: Value,
}

/// A trait that defines the functionality of a command.
/// Every command must implement this trait.
pub trait CommandExecutor {
	/// Execute the command with the arguments provided
	fn execute(
		self,
		global_args: &GlobalArgs,
	) -> impl Future<Output = anyhow::Result<CommandOutput>>;
}

/// A list of all possible output types generated by the CLI.
#[derive(Debug, Clone, Default, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum OutputType {
	/// A plain text output.
	#[default]
	Text,
	/// A JSON output, minified.
	Json,
	/// A JSON output, pretty printed.
	PrettyJson,
}

impl Display for OutputType {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			OutputType::Text => write!(f, "text"),
			OutputType::Json => write!(f, "json"),
			OutputType::PrettyJson => write!(f, "pretty-json"),
		}
	}
}
