use std::str::FromStr;

use models::{ApiSuccessResponseBody, api::auth::*, prelude::*};

use crate::prelude::*;

/// A command that logs the user out of their Patr account.
pub(super) async fn execute(args: GlobalArgs, state: AppState) -> Result<CommandOutput, AppError> {
	if args.token.is_some() {
		return CommandOutput::builder()
			.text(concat!(
				"You are logged in with an API token. You cannot log out. ",
				"If you would like to delete your API token, you can do so at ",
				"https://app.patr.cloud/user/api-token"
			))
			.json(ApiSuccessResponseBody::empty().to_json_value())
			.build()
			.into_result();
	}

	let (_, refresh_token) = match state {
		AppState::LoggedOut => {
			return CommandOutput::builder()
				.text("You are already logged out.")
				.json(ApiSuccessResponseBody::empty().to_json_value())
				.build()
				.into_result();
		}
		AppState::LoggedIn {
			token,
			refresh_token,
			current_workspace: _,
		} => (token, refresh_token),
	};

	LogoutResponse = make_request(
		ApiRequest::<LogoutRequest>::builder()
			.path(LogoutPath)
			.headers(LogoutRequestHeaders {
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
				refresh_token: BearerToken::from_str(refresh_token.as_str())?,
			})
			.query(())
			.body(LogoutRequest)
			.build(),
	)
	.await?
	.body;

	CommandOutput::builder()
		.text("You have been logged out.")
		.json(ApiSuccessResponseBody::empty().to_json_value())
		.build()
		.into_result()
}
