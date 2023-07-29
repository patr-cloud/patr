use std::io::Write;

use clap::Args;

use super::GlobalArgs;

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
	global_args: &GlobalArgs,
	args: LoginArgs,
	mut writer: impl Write + Send,
) -> Result<(), anyhow::Error> {
	let table = crate::models::commands::login::Table {
		first_name: "Test".to_string(),
		last_name: "User".to_string(),
		username: "testuser".to_string(),
	}
	.into_formatted();
	write!(writer, "{}", table)?;
	todo!()
}
