use std::str::FromStr;

use models::{ApiSuccessResponseBody, api::user::*, prelude::*};

use crate::{prelude::*, utils::oauth};

/// A command that logs the user into their Patr account.
///
/// Two ways in. `--token` takes an API token, which is what CI does: there is
/// no browser to open and no user to consent. Everything else goes through the
/// OAuth flow, which is a real login — the resulting session is tied to the
/// user's account, shows up under their authorized apps, and can be revoked
/// from the dashboard.
pub(super) async fn execute(
	global_args: GlobalArgs,
	mut state: AppState,
) -> Result<CommandOutput, AppError> {
	let (token, refresh_token, token_expiry) = match global_args.token {
		// An API token never expires and cannot be refreshed, so it is stored
		// on its own.
		Some(token) => (token, None, None),
		None => {
			let tokens = oauth::login().await?;
			(
				tokens.access_token,
				tokens.refresh_token,
				Some(tokens.expiry),
			)
		}
	};

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
		refresh_token,
		token_expiry,
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
