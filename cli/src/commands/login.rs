use std::{io::IsTerminal, str::FromStr};

use clap::Args as ClapArgs;
use inquire::{
	Password,
	Text,
	validator::{ErrorMessage, Validation},
};
use models::{
	ApiErrorResponseBody,
	ApiSuccessResponseBody,
	api::{auth::*, user::*},
	prelude::*,
};

use crate::prelude::*;

/// The arguments that can be passed to the login command.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// Email address or username to login with. Use `patr` as a username to
	/// login with your API token as a password.
	#[arg(short = 'u', long = "username", alias = "email")]
	pub user_id: Option<String>,
	/// The password to login with. If you are using an API token, use `patr`
	/// as the username and your API token as the password.
	#[arg(short = 'p', long)]
	pub password: Option<String>,
	/// The OTP provided by the MFA method, if any
	#[arg(long = "mfa")]
	pub mfa_otp: Option<String>,
}

/// A command that logs the user into their Patr account.
pub(super) async fn execute(
	args: Args,
	global_args: GlobalArgs,
	_: AppState,
) -> Result<CommandOutput, AppError> {
	// If there is a token provided, we will use that to login instead of the
	// username and password.
	if let Some(token) = &global_args.token {
		let response = make_request(
			ApiRequest::<GetUserInfoRequest>::builder()
				.path(GetUserInfoPath)
				.query(())
				.body(GetUserInfoRequest)
				.headers(GetUserInfoRequestHeaders {
					user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
					authorization: BearerToken::from_str(token)?,
				})
				.build(),
		)
		.await?;

		return CommandOutput::builder()
			.text(format!(
				"Logged in with an API token as `{}`. Hello {} {}!",
				response.body.basic_user_info.data.username,
				response.body.basic_user_info.data.first_name,
				response.body.basic_user_info.data.last_name
			))
			.json(ApiSuccessResponseBody::empty().to_json_value())
			.build()
			.into_result();
	}

	// If there is no token provided, we will use the username and password to
	// login. If the user is not logged in, we will prompt them for their
	// username and password. But we can't do that if the user is not using an
	// interactive terminal.
	if !std::io::stdin().is_terminal() {
		eprintln!(concat!(
			"In order to login to Patr, you either need to use an interactive terminal, ",
			"or provide an API token with the `--token` flag using an API token generated ",
			"at https://app.patr.cloud/user/api-token"
		));
		std::process::ExitCode::FAILURE.exit_process();
	}

	let user_id = args.user_id.unwrap_or_else(|| {
		Text::new("Username or email address:")
			.with_help_message("The email address or username used to log into Patr")
			.prompt()
			.expect_tty("Unable to read username or email address")
	});
	let password = args.password.unwrap_or_else(|| {
		Password::new("Password")
			.with_help_message("The password used to log into Patr")
			.without_confirmation()
			.prompt()
			.expect_tty("Unable to read password")
	});

	let response = make_request(
		ApiRequest::<LoginRequest>::builder()
			.query(())
			.headers(LoginRequestHeaders {
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
			})
			.path(LoginPath)
			.body(LoginRequest {
				user_id: user_id.clone(),
				password: password.clone(),
				mfa_otp: None,
			})
			.build(),
	)
	.await;

	let response = if let Err(ApiErrorResponse {
		body: ApiErrorResponseBody {
			error: ErrorType::MfaRequired,
			..
		},
		..
	}) = response
	{
		let mfa_otp = args.mfa_otp.unwrap_or_else(|| {
			Text::new("Two-factor authentication code")
				.with_help_message("The OTP provided by the MFA method")
				.with_validator(|value: &str| {
					if !value
						.chars()
						.filter(|char| *char != '-')
						.all(|c| c.is_ascii_digit())
					{
						return Ok(Validation::Invalid(ErrorMessage::Custom(
							"The OTP must be a 6 or 7 digit number".to_string(),
						)));
					}

					if value.chars().filter(|char| *char != '-').count() != 6 {
						return Ok(Validation::Invalid(ErrorMessage::Custom(
							"The OTP must be a 6 digit number".to_string(),
						)));
					}

					Ok(Validation::Valid)
				})
				.prompt()
				.expect_tty("Unable to read MFA OTP")
		});
		make_request(
			ApiRequest::<LoginRequest>::builder()
				.query(())
				.headers(LoginRequestHeaders {
					user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
				})
				.path(LoginPath)
				.body(LoginRequest {
					user_id,
					password,
					mfa_otp: Some(mfa_otp),
				})
				.build(),
		)
		.await
	} else {
		response
	};

	let LoginResponse {
		access_token,
		refresh_token,
	} = response?.body;

	let token = BearerToken::from_str(&access_token)?;

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
			.path(GetUserInfoPath)
			.query(())
			.headers(GetUserInfoRequestHeaders {
				authorization: token.clone(),
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
			})
			.body(GetUserInfoRequest)
			.build(),
	)
	.await?
	.body;

	let current_workspace = make_request(
		ApiRequest::<ListUserWorkspacesRequest>::builder()
			.path(ListUserWorkspacesPath)
			.headers(ListUserWorkspacesRequestHeaders {
				authorization: token.clone(),
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
			})
			.query(())
			.body(ListUserWorkspacesRequest)
			.build(),
	)
	.await?
	.body
	.workspaces
	.into_iter()
	.next()
	.map(|workspace| workspace.id);

	AppState::LoggedIn {
		token,
		refresh_token: refresh_token.clone(),
		current_workspace,
	}
	.save()?;

	CommandOutput::builder()
		.text(format!(
			"Logged in as `{username}`. Hello {first_name} {last_name}!"
		))
		.json(
			ApiSuccessResponseBody::new(LoginResponse {
				access_token,
				refresh_token,
			})
			.to_json_value(),
		)
		.build()
		.into_result()
}
