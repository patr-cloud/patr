use std::str::FromStr;

use inquire::Password;
use models::{ApiSuccessResponseBody, api::user::*, prelude::*};

use crate::prelude::*;

/// A command that logs the user into their Patr account.
pub(super) async fn execute(
	global_args: GlobalArgs,
	mut state: AppState,
) -> Result<CommandOutput, AppError> {
	// Prompt for the token
	let token = global_args.token.unwrap_or_else(|| {
		let token_url = format!("{}/profile/api-tokens/new", constants::FRONTEND_BASE_URL);

		eprintln!("Opening your browser to create a new API token...");
		match open::that(&token_url) {
			Ok(()) => eprintln!("Opened '{}' successfully.", token_url),
			Err(_err) => {
				eprintln!("If the browser did not open, please visit '{}'", token_url)
			}
		}

		Password::new("Paste your API token here:")
			.with_help_message("Create an API token in your browser and paste it here")
			.without_confirmation()
			.prompt()
			.expect_tty("Failed to read API token")
	});

	// Verify the token by fetching user info
	let GetUserInfoResponse {
		basic_user_info:
			WithId {
				id: _,
				data: BasicUserInfo {
					username,
					first_name,
					last_name,
				},
			},
		..
	} = make_request(
		ApiRequest::<GetUserInfoRequest>::builder()
			.headers(GetUserInfoRequestHeaders {
				user_agent: constants::USER_AGENT,
				authorization: BearerToken::from_str(&token)?,
			})
			.build(),
	)
	.await?
	.body;

	// Get the user's first workspace
	let current_workspace = make_request(
		ApiRequest::<ListUserWorkspacesRequest>::builder()
			.headers(ListUserWorkspacesRequestHeaders {
				authorization: BearerToken::from_str(&token)?,
				user_agent: constants::USER_AGENT,
			})
			.build(),
	)
	.await?
	.body
	.workspaces
	.into_iter()
	.next()
	.map(|workspace| workspace.id);

	// Save the authenticated state (preserving any existing target_channel).
	state.auth = AuthState::LoggedIn {
		token: BearerToken::from_str(&token)?,
		current_workspace,
	};
	state.save()?;

	CommandOutput::builder()
		.text(format!(
			"Logged in as `{username}`. Hello {first_name} {last_name}!"
		))
		.json(ApiSuccessResponseBody::empty().to_json_value())
		.build()
		.into_result()
}
