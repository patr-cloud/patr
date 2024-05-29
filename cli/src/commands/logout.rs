use std::str::FromStr;

use models::{api::auth::*, prelude::*, ApiErrorResponse, ApiSuccessResponseBody};

use crate::prelude::*;

/// A command that logs the user out of their Patr account.
pub(super) async fn execute(
	args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, ApiErrorResponse> {
	if args.token.is_some() {
		return CommandOutput {
			text: concat!(
				"You are logged in with an API token. You cannot log out. ",
				"If you would like to delete your API token, you can do so at ",
				"https://app.patr.cloud/user/api-token"
			)
			.to_string(),
			json: ApiSuccessResponseBody::empty().to_json_value(),
		}
		.into_result();
	}

	let (_, refresh_token) = match state {
		AppState::LoggedOut => {
			return CommandOutput {
				text: "You are already logged out.".to_string(),
				json: ApiSuccessResponseBody::empty().to_json_value(),
			}
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

	CommandOutput {
		text: "You have been logged out.".to_string(),
		json: ApiSuccessResponseBody::empty().to_json_value(),
	}
	.into_result()
}
