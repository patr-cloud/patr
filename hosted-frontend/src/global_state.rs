use leptos::*;

/// WIP NEEDS A BIT OF REWORK

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct AuthTokens {
	auth_token: String,
	refresh_token: String,
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

/// Returns the current auth state, and a setter that can be used to set the
/// auth token If `None` is passed into the setter, sets the AuthState as
/// LoggedOut, and if Some(AuthState) is passed, sets the refresh and auth
/// tokens
pub fn get_auth_state() -> (Signal<AuthState>, SignalSetter<Option<AuthTokens>>) {
	let state = use_context::<RwSignal<AuthState>>().expect("State Needs to be provided");
	let (state, set_state) = create_slice(
		state,
		|state| state.clone(),
		|state: &mut AuthState, auth_tokens: Option<AuthTokens>| {
			if let Some(AuthTokens {
				auth_token: token,
				refresh_token,
			}) = auth_tokens
			{
				*state = AuthState::LoggedIn(AuthTokens {
					auth_token: token,
					refresh_token,
				})
			} else {
				*state = AuthState::LoggedOut
			}
		},
	);

	(state, set_state)
}
