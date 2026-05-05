use crate::{prelude::*, utils::constants::USERNAME_VALIDITY_REGEX};

macros::declare_api_endpoint!(
	/// Creates a new Patr account from a GitHub identity after the user has
	/// confirmed/edited the pre-filled profile details. The setup_token was
	/// returned by the callback endpoint. Returns Patr tokens on success.
	GithubOAuthSetup,
	POST "/auth/social-login/github/setup",
	api = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The setup token returned by POST /auth/social-login/github/callback
		#[preprocess(trim, length(min = 1))]
		pub setup_token: String,
		/// The chosen Patr username
		#[preprocess(trim, length(min = 2), regex = USERNAME_VALIDITY_REGEX)]
		pub username: String,
		/// The user's first name
		#[preprocess(trim, length(min = 1))]
		pub first_name: String,
		/// The user's last name
		#[preprocess(trim, length(min = 1))]
		pub last_name: String,
	},
	response = {
		/// Patr JWT access token
		pub access_token: String,
		/// Patr refresh token
		pub refresh_token: String,
	},
	audit_log = NoAuditLogger,
);
