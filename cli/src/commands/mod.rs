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
/// The command to uninstall the Patr CLI from this host.
mod uninstall;
/// The command to upgrade the Patr CLI in place.
mod upgrade;
/// All commands that are meant for a workspace.
mod workspaced;

/// A list of all the arguments that can be passed to the CLI.
#[derive(Debug, Clone, Parser)]
#[command(author, version = build_version(), about)]
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
	/// Upgrade the Patr CLI in place.
	#[command(alias = "update", alias = "self-update")]
	Upgrade(upgrade::Args),
	/// Uninstall the Patr CLI from this host.
	#[command(alias = "delete")]
	Uninstall(uninstall::Args),
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
		GlobalCommand::Upgrade(args) => upgrade::execute(args, global_args, state).await,
		GlobalCommand::Uninstall(args) => uninstall::execute(args, global_args, state).await,
		GlobalCommand::Workspaced(commands) => {
			workspaced::execute(commands, global_args, state).await
		}
	}
}

/// Builds the version string shown by `patr --version`.
///
/// CI bakes `PATR_BUILD_VERSION` (full semver, e.g. `0.19.0-alpha.142`),
/// `PATR_BUILD_CHANNEL`, `PATR_BUILD_SHA`, and `PATR_BUILD_DATE` into the
/// binary. Local builds fall back to `CARGO_PKG_VERSION-dev`.
fn build_version() -> String {
	let version = constants::PATR_BUILD_VERSION;
	let channel = option_env!("PATR_BUILD_CHANNEL");
	let sha = option_env!("PATR_BUILD_SHA");
	let date = option_env!("PATR_BUILD_DATE");
	let os = std::env::consts::OS;
	let arch = std::env::consts::ARCH;

	if channel.is_none() {
		return format!("{version}-dev {os}/{arch}");
	}

	match (sha, date) {
		(Some(sha), Some(date)) => format!("{version} ({sha} {date}) {os}/{arch}"),
		(None, Some(date)) => format!("{version} ({date}) {os}/{arch}"),
		_ => format!("{version} {os}/{arch}"),
	}
}
