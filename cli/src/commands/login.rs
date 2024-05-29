use std::{io::IsTerminal, str::FromStr};

use clap::Args;
use models::{
	api::{auth::*, user::*},
	prelude::*,
	ApiErrorResponse,
	ApiSuccessResponseBody,
};

use crate::prelude::*;

/// The arguments that can be passed to the login command.
#[derive(Debug, Clone, Args)]
pub struct LoginArgs {
	/// Email address or username to login with. Use `patr` as a username to
	/// login with your API token as a password.
	#[arg(short = 'u', long = "username", alias = "email")]
	pub user_id: String,
	/// The password to login with. If you are using an API token, use `patr`
	/// as the username and your API token as the password.
	#[arg(short = 'p', long)]
	pub password: String,
	/// The OTP provided by the MFA method, if any
	#[arg(long = "mfa")]
	pub mfa_otp: Option<String>,
}

/// A command that logs the user into their Patr account.
pub(super) async fn execute(
	args: LoginArgs,
	global_args: GlobalArgs,
	_: AppState,
) -> Result<CommandOutput, ApiErrorResponse> {
	if let Some(ref token) = global_args.token {
		make_request(
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
	}

	if !std::io::stdin().is_terminal() {
		eprintln!(concat!(
			"In order to login to Patr, you either need to use an interactive terminal, ",
			"or provide an API token with the `--token` flag using an API token generated ",
			"at https://app.patr.cloud/user/api-token"
		));
		std::process::ExitCode::FAILURE.exit_process();
	}

	let LoginResponse {
		access_token,
		refresh_token,
	} = make_request(
		ApiRequest::<LoginRequest>::builder()
			.query(())
			.headers(LoginRequestHeaders {
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
			})
			.path(LoginPath)
			.body(LoginRequest {
				user_id: args.user_id,
				password: args.password,
				mfa_otp: args.mfa_otp,
			})
			.build(),
	)
	.await?
	.body;

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

	CommandOutput {
		text: format!("Logged in as `{username}`. Hello {first_name} {last_name}!"),
		json: ApiSuccessResponseBody::new(LoginResponse {
			access_token,
			refresh_token,
		})
		.to_json_value(),
	}
	.into_result()
}
