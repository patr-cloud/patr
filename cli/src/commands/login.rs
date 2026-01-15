use std::{io::IsTerminal, str::FromStr};

use clap::Args as ClapArgs;
use inquire::Password;
use models::{ApiSuccessResponseBody, api::user::*, prelude::*};

use crate::prelude::*;

/// The arguments that can be passed to the login command.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// The API token to login with. If not provided, you will be redirected
	/// to create one in your browser.
	#[arg(short = 't', long = "token")]
	pub token: Option<String>,
}

/// A command that logs the user into their Patr account.
pub(super) async fn execute(
	args: Args,
	global_args: GlobalArgs,
	_: AppState,
) -> Result<CommandOutput, AppError> {
	// Determine the base URL for the app
	let app_base_url = if cfg!(debug_assertions) {
		"http://localhost:3001"
	} else {
		"https://app.patr.cloud"
	};

	// Check if a token is provided via args or global args
	let token: Option<String> = args
		.token
		.or_else(|| global_args.token.clone())
		.or_else(|| {
			// If no token provided and we're in an interactive terminal,
			// open the browser to the API token creation page
			if std::io::stdin().is_terminal() {
				let token_url = format!("{}/profile/api-tokens/new", app_base_url);

				println!("Opening your browser to create a new API token...");
				match open::that(&token_url) {
					Ok(()) => println!("Opened '{}' successfully.", token_url),
					Err(err) => {
						eprintln!("An error occurred when opening '{}': {}", token_url, err)
					}
				}
				println!("URL: {}", token_url);

				// Try to open the browser, but don't fail if it doesn't work
				if let Err(e) = open::that(&token_url) {
					eprintln!("Failed to open browser: {}", e);
					eprintln!("Please manually visit: {}", token_url);
				}

				println!();

				// Prompt for the token
				let token_input = Password::new("Paste your API token here:")
					.with_help_message("Create an API token in your browser and paste it here")
					.without_confirmation()
					.prompt()
					.expect_tty("Unable to read API token");

				Some(token_input)
			} else {
				None
			}
		});

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
		refresh_token: String::new(), // API tokens don't have refresh tokens
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
