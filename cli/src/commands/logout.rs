use models::ApiSuccessResponseBody;

use crate::{prelude::*, utils::oauth};

/// A command that logs the user out of their Patr account.
pub(super) async fn execute(
	_args: GlobalArgs,
	mut state: AppState,
) -> Result<CommandOutput, AppError> {
	let AuthState::LoggedIn { refresh_token, .. } = &state.auth else {
		return CommandOutput::builder()
			.text("You are not logged in.")
			.json(ApiSuccessResponseBody::empty().to_json_value())
			.build()
			.into_result();
	};

	// Tell the API to end the grant, so the session disappears from the user's
	// authorized apps too. Best-effort — see `oauth::revoke`.
	let text = match refresh_token {
		Some(refresh_token) => {
			oauth::revoke(refresh_token).await;
			"You have been logged out."
		}
		// An API token is not ours to revoke: the user created it in the
		// dashboard and may be using it elsewhere.
		None => "You have been logged out. Don't forget to revoke your API token!",
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
