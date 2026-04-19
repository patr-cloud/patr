use models::ApiSuccessResponseBody;

use crate::prelude::*;

/// A command that logs the user out of their Patr account.
pub(super) async fn execute(
	_args: GlobalArgs,
	mut state: AppState,
) -> Result<CommandOutput, AppError> {
	if !state.is_logged_in() {
		return CommandOutput::builder()
			.text("You are not logged in.")
			.json(ApiSuccessResponseBody::empty().to_json_value())
			.build()
			.into_result();
	}

	// Drop the auth while preserving any other preferences (target_channel etc).
	state.auth = AuthState::LoggedOut {};
	state.save()?;

	CommandOutput::builder()
		.text("You have been logged out. Don't forget to revoke your API token!")
		.json(ApiSuccessResponseBody::empty().to_json_value())
		.build()
		.into_result()
}
