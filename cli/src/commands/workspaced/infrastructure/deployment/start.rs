use clap::Args;
use inquire::Text;

use crate::prelude::*;

#[derive(Debug, Clone, Args)]
pub struct StartArgs {
	/// The name of the deployment
	#[arg(
		short = 'n',
		long = "name",
		value_name = "NAME",
		env = "PATR_DEPLOYMENT_NAME"
	)]
	pub name: Option<String>,
}

pub async fn execute(
	args: StartArgs,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	let AppState::LoggedIn {
		token,
		refresh_token: _,
		current_workspace,
	} = state
	else {
		return Err(AppError::NotLoggedIn);
	};

	let deployment_id = args.name.unwrap_or_else(|| {
		Text::new("Please enter the deployment you want to start:")
			.prompt()
			.expect_tty("Failed to read deployment ID")
	});

	Err(AppError::NotLoggedIn)
}
