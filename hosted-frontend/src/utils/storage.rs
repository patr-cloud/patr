use axum_extra::extract::cookie::SameSite;
use leptos::*;
use leptos_use::{use_cookie_with_options, utils::FromToStringCodec, UseCookieOptions};

use crate::prelude::*;

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
		let access_token = use_cookie_with_options::<String, FromToStringCodec>(
			constants::ACCESS_TOKEN,
			UseCookieOptions::default()
				.secure(!cfg!(debug_assertions))
				.http_only(true)
				.same_site(SameSite::Strict),
		)
		.0
		.get_untracked();

		let refresh_token = use_cookie_with_options::<String, FromToStringCodec>(
			constants::REFRESH_TOKEN,
			UseCookieOptions::default()
				.secure(!cfg!(debug_assertions))
				.http_only(true)
				.same_site(SameSite::Strict),
		)
		.0
		.get_untracked();

		let last_used_workspace_id = use_cookie_with_options::<String, FromToStringCodec>(
			constants::LAST_USED_WORKSPACE_ID,
			UseCookieOptions::default()
				.secure(!cfg!(debug_assertions))
				.http_only(true)
				.same_site(SameSite::Strict),
		)
		.0
		.get_untracked()
		.and_then(|id| Uuid::parse_str(&id).ok());

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
		#[cfg(not(target_arch = "wasm32"))]
		let cookie_setter = |cookie: &cookie::Cookie| {
			let response = expect_context::<leptos_axum::ResponseOptions>();
			response.append_header(
				http::header::SET_COOKIE,
				http::HeaderValue::from_str(&cookie.to_string()).unwrap(),
			);
		};
		#[cfg(target_arch = "wasm32")]
		let cookie_setter = |cookie: &cookie::Cookie| {};
		match self {
			AuthState::LoggedOut => {
				use_cookie_with_options::<String, FromToStringCodec>(
					constants::ACCESS_TOKEN,
					UseCookieOptions::<String, _>::default()
						.secure(!cfg!(debug_assertions))
						.http_only(true)
						.same_site(SameSite::Strict)
						.ssr_set_cookie(cookie_setter),
				)
				.1
				.set(None);

				use_cookie_with_options::<String, FromToStringCodec>(
					constants::REFRESH_TOKEN,
					UseCookieOptions::<String, _>::default()
						.secure(!cfg!(debug_assertions))
						.http_only(true)
						.same_site(SameSite::Strict)
						.ssr_set_cookie(cookie_setter),
				)
				.1
				.set(None);

				use_cookie_with_options::<String, FromToStringCodec>(
					constants::LAST_USED_WORKSPACE_ID,
					UseCookieOptions::<String, _>::default()
						.secure(!cfg!(debug_assertions))
						.http_only(true)
						.same_site(SameSite::Strict)
						.ssr_set_cookie(cookie_setter),
				)
				.1
				.set(None);
			}
			AuthState::LoggedIn {
				access_token,
				refresh_token,
				last_used_workspace_id,
			} => {
				use_cookie_with_options::<String, FromToStringCodec>(
					constants::ACCESS_TOKEN,
					UseCookieOptions::<String, _>::default()
						.secure(!cfg!(debug_assertions))
						.http_only(true)
						.same_site(SameSite::Strict)
						.ssr_set_cookie(cookie_setter),
				)
				.1
				.set(Some(access_token));

				use_cookie_with_options::<String, FromToStringCodec>(
					constants::REFRESH_TOKEN,
					UseCookieOptions::<String, _>::default()
						.secure(!cfg!(debug_assertions))
						.http_only(true)
						.same_site(SameSite::Strict)
						.ssr_set_cookie(cookie_setter),
				)
				.1
				.set(Some(refresh_token));

				use_cookie_with_options::<String, FromToStringCodec>(
					constants::LAST_USED_WORKSPACE_ID,
					UseCookieOptions::<String, _>::default()
						.secure(!cfg!(debug_assertions))
						.http_only(true)
						.same_site(SameSite::Strict)
						.ssr_set_cookie(cookie_setter),
				)
				.1
				.set(last_used_workspace_id.map(|id| id.to_string()));
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
