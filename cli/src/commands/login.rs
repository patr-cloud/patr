use clap::Args;

use super::GlobalArgs;
use crate::CommandOutput;

/// The arguments that can be passed to the login command.
#[derive(Debug, Clone, Args)]
pub struct LoginArgs {
	/// Email address or username to login with. Use `patr` as a username to
	/// login with your API token as a password.
	#[arg(short = 'u', long = "username", alias = "email")]
	pub user_id: String,
	/// The password to login with. If you are using an API token, use `patr`
	/// as the username and your API token as the password.
	#[arg(short = 'p', long)]
	pub password: String,
}

/// A command that logs the user into their Patr account.
pub(super) async fn execute(
	_global_args: &GlobalArgs,
	_args: LoginArgs,
) -> Result<CommandOutput, anyhow::Error> {
	todo!()
}
