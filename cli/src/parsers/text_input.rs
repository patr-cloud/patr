use clap::builder::{IntoResettable, OsStr, Resettable};
use inquire::Text;
use typed_builder::TypedBuilder;

/// A parser for text input. This is used to prompt the user for input
/// in a terminal. It is used for the username fields.
#[derive(Debug, Clone, TypedBuilder)]
pub struct TextInputParser {
	/// The prompt to display to the user when asking for input.
	#[builder(setter(into))]
	pub prompt: String,
	/// The placeholder to display to the user when asking for input.
	#[builder(default, setter(into))]
	pub placeholder: String,
	#[builder(default = super::DEFAULT_NO_TTY_MESSAGE.to_string(), setter(into))]
	pub no_tty_message: String,
}

impl IntoResettable<OsStr> for TextInputParser {
	fn into_resettable(self) -> Resettable<OsStr> {
		let Ok(string) = Text::new(&self.prompt)
			.with_placeholder(&self.placeholder)
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
