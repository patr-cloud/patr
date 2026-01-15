use std::str::FromStr;

use inquire::Password;
use models::{ApiSuccessResponseBody, api::user::*, prelude::*};

use crate::prelude::*;

/// A command that logs the user into their Patr account.
pub(super) async fn execute(_: AppState) -> Result<CommandOutput, AppError> {
	// Determine the base URL for the app
	let app_base_url = std::env::var("FRONTEND_BASE_URL");

	let app_base_url = if cfg!(debug_assertions) {
		app_base_url.unwrap_or_default()
	} else {
		app_base_url.expect("FRONTEND_BASE_URL environment variable is not set")
	};

	let token_url = format!("{}/profile/api-tokens/new", app_base_url);

	println!("Opening your browser to create a new API token...");
	match open::that(&token_url) {
		Ok(()) => println!("Opened '{}' successfully.", token_url),
		Err(_err) => {
			println!("If the browser did not open, please visit '{}'", token_url)
		}
	}

	println!();

	// Prompt for the token
	let token: Option<String> = Password::new("Paste your API token here:")
		.with_help_message("Create an API token in your browser and paste it here")
		.without_confirmation()
		.prompt()
		.ok();

	// If we still don't have a token, exit with an error
	let token = match token {
		Some(t) => t,
		None => {
			eprintln!(
				concat!(
					"In order to login to Patr, you need to provide an API token.\n",
					"You can either:\n",
					"  1. Run this command in an interactive terminal\n",
					"  2. Provide the token with the `--token` flag\n",
					"  3. Create an API token at {}/profile/api-tokens/new"
				),
				app_base_url
			);
			std::process::ExitCode::FAILURE.exit_process();
		}
	};

	// Verify the token by fetching user info
	let response = make_request(
		ApiRequest::<GetUserInfoRequest>::builder()
			.headers(GetUserInfoRequestHeaders {
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
				authorization: BearerToken::from_str(&token)?,
			})
			.build(),
	)
	.await?;

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
	} = response.body;

	// Get the user's first workspace
	let current_workspace = make_request(
		ApiRequest::<ListUserWorkspacesRequest>::builder()
			.headers(ListUserWorkspacesRequestHeaders {
				authorization: BearerToken::from_str(&token)?,
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
			})
			.build(),
	)
	.await?
	.body
	.workspaces
	.into_iter()
	.next()
	.map(|workspace| workspace.id);

	// Save the authenticated state
	AppState::LoggedIn {
		token: BearerToken::from_str(&token)?,
		current_workspace,
	}
	.save()?;

	CommandOutput::builder()
		.text(format!(
			"Logged in as `{username}`. Hello {first_name} {last_name}!"
		))
		.json(ApiSuccessResponseBody::empty().to_json_value())
		.build()
		.into_result()
}
