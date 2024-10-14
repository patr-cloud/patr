use models::api::auth::*;

use crate::prelude::*;

/// Server function for completing the sign up process
#[server(ConfirmOtp, endpoint = "auth/join")]
async fn complete_sign_up(
	username: String,
	otp: String,
) -> Result<CompleteSignUpResponse, ServerFnError<ErrorType>> {
	make_api_call::<CompleteSignUpRequest>(
		ApiRequest::builder()
			.path(CompleteSignUpPath)
			.query(())
			.headers(CompleteSignUpRequestHeaders {
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(CompleteSignUpRequest {
				username,
				verification_token: otp,
			})
			.build(),
	)
	.await
	.map(|res| {
		AuthState::load().1.set(Some(AuthState::LoggedIn {
			access_token: res.body.access_token.clone(),
			refresh_token: res.body.refresh_token.clone(),
			last_used_workspace_id: None,
		}));
		leptos_axum::redirect("/");
		res.body
	})
	.map_err(ServerFnError::WrappedServerError)
}
