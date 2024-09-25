use models::api::auth::*;

use crate::prelude::*;

/// Server Function to sign up a new user
#[server(CreateAccount, endpoint = "auth/sign-up")]
pub async fn sign_up(
	username: String,
	password: String,
	first_name: String,
	last_name: String,
	email: String,
) -> Result<CreateAccountResponse, ServerFnError<ErrorType>> {
	make_request::<CreateAccountRequest>(
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
	.map(|res| res.body)
}
