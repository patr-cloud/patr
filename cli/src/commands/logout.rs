use models::ApiSuccessResponseBody;

use crate::prelude::*;

/// A command that logs the user out of their Patr account.
pub(super) async fn execute(_args: GlobalArgs, state: AppState) -> Result<CommandOutput, AppError> {
	let AppState::LoggedIn { token, .. } = state else {
		return CommandOutput::builder()
			.text("You are not logged in.")
			.json(ApiSuccessResponseBody::empty().to_json_value())
			.build()
			.into_result();
	};

	// Save the logged out state
	AppState::LoggedOut.save()?;

	CommandOutput::builder()
		.text("You have been logged out and your API token has been revoked.")
		.json(ApiSuccessResponseBody::empty().to_json_value())
		.build()
		.into_result()
}
