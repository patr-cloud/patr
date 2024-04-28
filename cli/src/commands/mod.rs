use clap::{Args, Parser, Subcommand};
use models::ApiErrorResponse;

use self::{login::LoginArgs, workspaced::WorkspacedCommands};
use crate::prelude::*;

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
	pub global_args: GlobalArgs,
	/// A command that is called on the CLI.
	#[command(subcommand)]
	pub command: GlobalCommands,
}

/// A global list of all the arguments that can be passed to the CLI.
#[derive(Debug, Clone, Args)]
pub struct GlobalArgs {
	/// The output type of each command. Defaults to text.
	#[arg(short = 'o', default_value_t = OutputType::Text)]
	pub output: OutputType,
	/// The token used to authenticate with the API, instead of the login
	/// credentials
	pub token: Option<String>,
}

/// A list of all the commands that can be called on the CLI.
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GlobalCommands {
	/// Login to your Patr account.
	#[command(alias = "signin", alias = "sign-in")]
	Login(LoginArgs),
	/// Logout of your Patr account.
	Logout,
	/// Get information about the current logged in user.
	#[command(alias = "whoami")]
	Info,
	/// All the commands that are meant for a workspace
	#[command(flatten)]
	Workspaced(WorkspacedCommands),
}

impl CommandExecutor for GlobalCommands {
	async fn execute(
		self,
		global_args: GlobalArgs,
		state: AppState,
	) -> Result<CommandOutput, ApiErrorResponse> {
		match self {
			Self::Login(args) => login::execute(args, global_args, state).await,
			Self::Logout => logout::execute(global_args, state).await,
			Self::Info => info::execute(global_args, state).await,
			Self::Workspaced(commands) => commands.execute(global_args, state).await,
		}
	}
}
