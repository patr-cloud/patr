use http::header::ACCEPT_ENCODING;
use leptos::*;
use leptos_use::{use_cookie, utils::FromToStringCodec};

/// WIP NEEDS A BIT OF REWORK

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct AuthTokens {
	pub auth_token: String,
	pub refresh_token: String,
}

#[derive(Default, Debug, Clone)]
pub struct GlobalState {
	pub auth_state: AuthState,
}

impl GlobalState {
	pub const fn new() -> Self {
		GlobalState {
			auth_state: AuthState::LoggedOut,
		}
	}

	pub const fn init() -> Self {
		GlobalState {
			auth_state: AuthState::LoggedOut,
		}
	}
}

/// The Auth state, contains the log in status and the auth token
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub enum AuthState {
	#[default]
	LoggedOut,
	LoggedIn(AuthTokens),
}

impl AuthState {
	/// Is the user logged in
	pub fn is_logged_in(&self) -> bool {
		match self {
			Self::LoggedOut => false,
			Self::LoggedIn(AuthTokens {
				auth_token: _,
				refresh_token: _,
			}) => true,
		}
	}
}
// -> Signal<AuthState>
pub fn authstate_from_cookie() -> Signal<AuthState> {
	let state = use_context::<RwSignal<GlobalState>>().expect("state to be provided");

	let (state, set_cookie_state) = create_slice(
		state,
		|state: &GlobalState| state.auth_state.clone(),
		move |state: &mut GlobalState, _: ()| {
			let (access_token, _) = use_cookie::<String, FromToStringCodec>("access_token");
			let (refresh_token, _) = use_cookie::<String, FromToStringCodec>("refresh_token");

			if let (Some(access_token), Some(refresh_token)) =
				(access_token.get_untracked(), refresh_token.get_untracked())
			{
				logging::log!("{} {}", access_token.clone(), refresh_token.clone());
				state.auth_state = AuthState::LoggedIn(AuthTokens {
					auth_token: access_token.clone(),
					refresh_token: refresh_token.clone(),
				})
			}
		},
	);

	set_cookie_state.set(());

	state
}

/// Returns the current auth state, and a setter that can be used to set the
/// auth token If `None` is passed into the setter, sets the AuthState as
/// LoggedOut, and if Some(AuthState) is passed, sets the refresh and auth
/// tokens
pub fn get_auth_state() -> (Signal<AuthState>, SignalSetter<Option<AuthTokens>>) {
	let state = use_context::<RwSignal<GlobalState>>().expect("State Needs to be provided");
	let (state, set_state) = create_slice(
		state,
		|state| state.auth_state.clone(),
		|state: &mut GlobalState, auth_tokens: Option<AuthTokens>| {
			if let Some(AuthTokens {
				auth_token: token,
				refresh_token,
			}) = auth_tokens
			{
				state.auth_state = AuthState::LoggedIn(AuthTokens {
					auth_token: token,
					refresh_token,
				})
			} else {
				state.auth_state = AuthState::LoggedOut
			}
		},
	);

	(state, set_state)
}

/// Returns a funciton that can be used to set the auth token
/// Make it directly take an Option<String>
pub fn set_auth_token() -> SignalSetter<Option<AuthTokens>> {
	let state = use_context::<RwSignal<GlobalState>>().expect("State Needs to be provided");
	let (_, set_state) = create_slice(
		state,
		|_| {},
		|state: &mut GlobalState, t: Option<AuthTokens>| {
			if let Some(token) = t {
				state.auth_state = AuthState::LoggedIn(AuthTokens {
					auth_token: token.auth_token,
					refresh_token: token.refresh_token,
				})
			} else {
				state.auth_state = AuthState::LoggedOut
			}
		},
	);

	set_state
}
