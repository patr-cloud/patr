use leptos::*;
use leptos_use::{use_cookie, utils::FromToStringCodec};

use crate::prelude::*;

/// The struct to store in the context for the auth state
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthStateContext(pub RwSignal<AuthState>);

/// The auth state stores the information about the user's login status, along
/// with the data associated with the login, if logged in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum AuthState {
	/// The user is logged out
	#[default]
	LoggedOut,
	/// The user is logged in
	LoggedIn {
		/// The JWT access token. Used to authenticate requests to the server
		/// and is stored in the browser cookies
		access_token: String,
		/// The Refresh Token, used to get a new access token when the current
		/// one expires or is invalid
		refresh_token: String,
		/// The workspace ID that was last used by the user. In case they switch
		/// workspaces, this is used to remember the last workspace they were in
		/// so that the page loads with that workspace by default.
		last_used_workspace_id: Option<Uuid>,
	},
}

impl AuthState {
	/// Load the auth state from the browser cookie storage. This is used to get
	/// the auth state when the app is first loaded
	pub fn load() -> Self {
		let access_token = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN)
			.0
			.get_untracked();

		let refresh_token = use_cookie::<String, FromToStringCodec>(constants::REFRESH_TOKEN)
			.0
			.get_untracked();

		let last_used_workspace_id =
			use_cookie::<Uuid, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID)
				.0
				.get_untracked();

		// TODO: Is a CSRF token needed?

		if let Some((access_token, refresh_token)) = access_token.zip(refresh_token) {
			AuthState::LoggedIn {
				access_token,
				refresh_token,
				last_used_workspace_id,
			}
		} else {
			AuthState::LoggedOut
		}
	}

	/// Save the auth state to the browser cookie storage. This is used to save
	/// the auth state when the user logs in or logs out
	pub fn save(self) {
		match self {
			AuthState::LoggedOut => {
				use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN)
					.1
					.set(None::<String>);

				use_cookie::<String, FromToStringCodec>(constants::REFRESH_TOKEN)
					.1
					.set(None);

				use_cookie::<Uuid, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID)
					.1
					.set(None);
			}
			AuthState::LoggedIn {
				access_token,
				refresh_token,
				last_used_workspace_id,
			} => {
				use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN)
					.1
					.set(Some(access_token));

				use_cookie::<String, FromToStringCodec>(constants::REFRESH_TOKEN)
					.1
					.set(Some(refresh_token));

				use_cookie::<Uuid, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID)
					.1
					.set(last_used_workspace_id);
			}
		}
	}

	/// Check if the user is logged in
	pub fn is_logged_in(&self) -> bool {
		matches!(self, AuthState::LoggedIn { .. })
	}

	/// Check if the user is logged out
	pub fn is_logged_out(&self) -> bool {
		matches!(self, AuthState::LoggedOut)
	}
}
