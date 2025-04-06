use clap::builder::{IntoResettable, OsStr, Resettable};
use inquire::Password;
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, TypedBuilder)]
pub struct PasswordInputParser {
	#[builder(setter(into))]
	pub prompt: String,
	#[builder(default, setter(into))]
	pub help: String,
	#[builder(default = super::DEFAULT_NO_TTY_MESSAGE.to_string(), setter(into))]
	pub no_tty_message: String,
}

impl IntoResettable<OsStr> for PasswordInputParser {
	fn into_resettable(self) -> Resettable<OsStr> {
		let Ok(string) = Password::new(&self.prompt)
			.with_help_message(&self.help)
			.without_confirmation()
			.prompt()
		else {
			eprintln!(concat!(
				"In order to login to Patr, you either need to use an interactive terminal, ",
				"or provide an API token with the `--token` flag using an API token generated ",
				"at https://app.patr.cloud/user/api-token"
			));
			std::process::ExitCode::FAILURE.exit_process();
		};
		Resettable::Value(string.into())
	}
}
