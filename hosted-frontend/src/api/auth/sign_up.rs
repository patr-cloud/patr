use models::api::auth::*;

use crate::prelude::*;

#[server(CreateAccount, endpoint = "auth/sign-up")]
pub async fn sign_up(
	username: String,
	password: String,
	first_name: String,
	last_name: String,
	email: String,
) -> Result<CreateAccountResponse, ServerFnError<ErrorType>> {
	make_api_call::<CreateAccountRequest>(
		ApiRequest::builder()
			.path(CreateAccountPath)
			.query(())
			.headers(CreateAccountRequestHeaders {
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(CreateAccountRequest {
				username,
				password,
				first_name,
				last_name,
				recovery_method: RecoveryMethod::Email {
					recovery_email: email,
				},
			})
			.build(),
	)
	.await
	.map(|res| {
		leptos_axum::redirect("/confirm");
		res.body
	})
	.map_err(ServerFnError::WrappedServerError)
}
