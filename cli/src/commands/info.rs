use comfy_table::Table;
use models::{api::user::*, prelude::*, ApiErrorResponse, ApiSuccessResponseBody};

use super::GlobalArgs;
use crate::prelude::*;

/// A command that gets information about the current logged in user.
pub(super) async fn execute(
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, ApiErrorResponse> {
	if global_args.token.is_some() {
		return CommandOutput {
			text: "The --token flag is not supported for this command.".to_owned(),
			json: ApiSuccessResponseBody::empty().to_json_value(),
		}
		.into_result();
	}

	let (access_token, _) = match state {
		AppState::LoggedIn {
			token,
			refresh_token,
			current_workspace: _,
		} => (token, refresh_token),
		AppState::LoggedOut => {
			return Err(ApiErrorResponse::error_with_message(
				ErrorType::Unauthorized,
				"You are not logged in. Run `patr login` to sign in to your Patr account."
					.to_owned(),
			));
		}
	};

	let GetUserInfoResponse {
		basic_user_info:
			WithId {
				id,
				data: BasicUserInfo {
					first_name,
					last_name,
					username,
				},
			},
		created,
		recovery_email,
		recovery_phone_number,
		is_mfa_enabled,
	} = make_request(
		ApiRequest::<GetUserInfoRequest>::builder()
			.path(GetUserInfoPath)
			.query(())
			.headers(GetUserInfoRequestHeaders {
				authorization: access_token.clone(),
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
			})
			.body(GetUserInfoRequest)
			.build(),
	)
	.await?
	.body;

	CommandOutput {
		text: Table::new()
			.set_header(["Data", "Value"])
			.add_row(["ID".to_owned(), id.to_string()])
			.add_row(["First Name", first_name.as_str()])
			.add_row(["Last Name", last_name.as_str()])
			.add_row(["Username", username.as_str()])
			.add_row(["Created At", created.to_string().as_str()])
			.add_row([
				"Recovery Email",
				recovery_email.as_deref().unwrap_or_default(),
			])
			.add_row([
				"Recovery Phone Number",
				recovery_phone_number
					.as_ref()
					.map(|number| format!("+{} {}", number.country_code, number.phone_number))
					.unwrap_or_default()
					.as_str(),
			])
			.add_row(["2FA Enabled", is_mfa_enabled.to_string().as_str()])
			.to_string(),
		json: GetUserInfoResponse {
			basic_user_info: WithId {
				id,
				data: BasicUserInfo {
					first_name: first_name.to_owned(),
					last_name: last_name.to_owned(),
					username: username.to_owned(),
				},
			},
			created,
			recovery_email,
			recovery_phone_number,
			is_mfa_enabled,
		}
		.to_json_value(),
	}
	.into_result()
}
