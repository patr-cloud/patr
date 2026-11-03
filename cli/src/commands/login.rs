use std::str::FromStr;

use models::{ApiSuccessResponseBody, api::user::*, prelude::*};

use crate::{prelude::*, utils::oauth};

/// A command that logs the user into their Patr account.
pub(super) async fn execute(
	global_args: GlobalArgs,
	mut state: AppState,
) -> Result<CommandOutput, AppError> {
	// `--token` takes an API token, for CI, which has nothing to refresh.
	// Otherwise the browser logs the user in.
	let (token, session) = match global_args.token {
		Some(token) => (token, None),
		None => {
			let tokens = oauth::login().await?;
			(tokens.access_token, Some(tokens.session))
		}
	};

	if token.starts_with("patr_rn_") {
		return Err(AppError::ParseError(
			"That's a runner token, which can't log in. To set a runner up with it, run \
			 `patr -w <workspace-id> runner setup reconnect --runner-id <runner-id> --runner-token <token>`."
				.to_string(),
		));
	}

	// Verify the token by fetching user info
	let GetUserInfoResponse {
		basic_user_info:
			WithId {
				id: _,
				data: BasicUserInfo {
					first_name,
					last_name,
				},
			},
		email,
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
		session,
	};
	state.save()?;

	CommandOutput::builder()
		.text(format!(
			"Logged in as `{email}`. Hello {first_name} {last_name}!"
		))
		.json(ApiSuccessResponseBody::empty().to_json_value())
		.build()
		.into_result()
}
