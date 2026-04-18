use clap::Subcommand;

use crate::prelude::*;

/// Create a new runner
mod create;
/// Print info about the runner configured on this host
mod current;
/// List deployments assigned to a specific runner
mod deployments;
/// List all runners
mod list;
/// The command to run a configured runner
mod run;
/// Systemd service lifecycle for the runner (install / uninstall / status)
mod service;
/// The command to setup the CLI's configuration settings for first time use.
mod setup;

/// A list of all the commands that can be called on a workspace.
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum RunnerCommand {
	/// The command that can be executed on runner
	#[command(subcommand, name = "runner")]
	RunnerAction(RunnerActionCommand),
	/// The command list all runners in a workspace
	#[command(name = "runners")]
	ListRunners,
}
/// The action that can be performed on a runner
#[derive(Debug, Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum RunnerActionCommand {
	/// Setup the CLI's configuration settings for first time use.
	#[command(alias = "configure")]
	Setup(setup::Args),
	/// Create a new runner by name
	#[command(alias = "new")]
	Create(create::Args),
	/// The command to list all runners for a specific workspace
	#[command(alias = "ls")]
	List,
	#[command(alias = "exec", alias = "execute", alias = "start")]
	/// Run a configured runner
	Run(run::Args),
	/// Systemd service lifecycle for the runner (install / uninstall / status)
	#[command(subcommand)]
	Service(service::ServiceCommand),
	/// Print info about the runner configured on this host
	#[command(alias = "whoami", alias = "info", alias = "status")]
	Current(current::Args),
	/// List deployments assigned to a runner
	#[command(alias = "list-deployments", alias = "ls-deployments")]
	Deployments(deployments::Args),
}

/// All commands that are executed on runner related stuff
pub async fn execute(
	command: RunnerCommand,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	use RunnerActionCommand::*;
	match command {
		RunnerCommand::RunnerAction(Setup(args)) => setup::execute(args, global_args, state).await,
		RunnerCommand::RunnerAction(Create(args)) => {
			create::execute(args, global_args, state).await
		}
		RunnerCommand::RunnerAction(List) | RunnerCommand::ListRunners => {
			list::execute(global_args, state).await
		}
		RunnerCommand::RunnerAction(Run(args)) => run::execute(args).await,
		RunnerCommand::RunnerAction(Service(cmd)) => service::execute(cmd).await,
		RunnerCommand::RunnerAction(Current(args)) => current::execute(args).await,
		RunnerCommand::RunnerAction(Deployments(args)) => {
			deployments::execute(args, global_args, state).await
		}
	}
}
