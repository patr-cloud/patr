use clap::{Args, Parser, Subcommand};

use self::workspaced::WorkspacedCommand;
use crate::prelude::*;

/// The command to apply a configuration to a workspace.
mod apply;
/// The command to get information about the current logged in user.
mod info;
/// The command to login to your Patr account.
mod login;
/// The command to logout of your Patr account.
mod logout;
/// All commands that are meant for a workspace.
mod workspaced;

/// A list of all the arguments that can be passed to the CLI.
#[derive(Debug, Clone, Parser)]
#[command(author, version, about)]
pub struct AppArgs {
	/// All global arguments that can be used across all commands.
	#[command(flatten)]
	pub args: GlobalArgs,
	/// A command that is called on the CLI.
	#[command(subcommand)]
	pub command: GlobalCommand,
}

/// A global list of all the arguments that can be passed to the CLI.
#[derive(Debug, Clone, Args)]
pub struct GlobalArgs {
	/// The output type of each command. Defaults to text.
	#[arg(
		short = 'o',
		long = "output-type",
		env = "PATR_OUTPUT_TYPE",
		default_value_t = OutputType::default(),
	)]
	pub output: OutputType,
	/// The token used to authenticate with the API, instead of the login
	/// credentials
	#[arg(short = 't', long = "token", env = "PATR_TOKEN")]
	pub token: Option<String>,
	/// The workspace to use for the command. If not specified, the current
	/// workspace will be used.
	#[arg(
		short = 'w',
		long = "workspace",
		value_name = "WORKSPACE-ID-OR-NAME",
		env = "PATR_WORKSPACE"
	)]
	pub workspace: Option<String>,
}

/// A list of all the commands that can be called on the CLI.
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GlobalCommand {
	/// Login to your Patr account.
	#[command(alias = "signin", alias = "sign-in")]
	Login,
	/// Logout of your Patr account.
	Logout,
	/// Get information about the current logged in user.
	#[command(alias = "whoami")]
	Info,
	/// Apply a configuration file to the current workspace.
	#[command(name = "apply")]
	Apply(apply::Args),
	/// All the commands that are meant for a workspace
	#[command(flatten)]
	Workspaced(WorkspacedCommand),
}

pub async fn execute(
	command: GlobalCommand,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	match command {
		GlobalCommand::Login => login::execute(global_args, state).await,
		GlobalCommand::Logout => logout::execute(global_args, state).await,
		GlobalCommand::Info => info::execute(global_args, state).await,
		GlobalCommand::Apply(args) => apply::execute(args, global_args, state).await,
		GlobalCommand::Workspaced(commands) => {
			workspaced::execute(commands, global_args, state).await
		}
	}
}
