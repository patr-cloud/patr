use models::ApiSuccessResponseBody;

use crate::{prelude::*, utils::oauth};

/// A command that logs the user out of their Patr account.
pub(super) async fn execute(
	_args: GlobalArgs,
	mut state: AppState,
) -> Result<CommandOutput, AppError> {
	let AuthState::LoggedIn { session, .. } = &state.auth else {
		return CommandOutput::builder()
			.text("You are not logged in.")
			.json(ApiSuccessResponseBody::empty().to_json_value())
			.build()
			.into_result();
	};

	// An API token isn't ours to revoke: the user may be using it elsewhere.
	let text = if let Some(session) = session {
		oauth::revoke(&session.refresh_token).await;
		"You have been logged out."
	} else {
		"You have been logged out. Don't forget to revoke your API token!"
	};

	// Drop the auth while preserving any other preferences (target_channel etc).
	state.auth = AuthState::LoggedOut {};
	state.save()?;

	CommandOutput::builder()
		.text(text)
		.json(ApiSuccessResponseBody::empty().to_json_value())
		.build()
		.into_result()
}
