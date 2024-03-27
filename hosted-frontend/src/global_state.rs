use leptos::*;

/// WIP NEEDS A BIT OF REWORK

/// The Auth state, contains the log in status and the auth token
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub enum AuthState {
	#[default]
	LoggedOut,
	LoggedIn {
		auth_token: String,
	},
}

impl AuthState {
	/// Is the user logged in
	pub fn is_logged_in(&self) -> bool {
		match self {
			Self::LoggedOut => false,
			Self::LoggedIn { auth_token: _ } => true,
		}
	}
}

/// Gets the logged in state and the auth token
pub fn get_auth_state() -> Signal<AuthState> {
	let state = use_context::<RwSignal<AuthState>>().expect("State Needs to be provided");
	let (state, _) = create_slice(state, |state| state.clone(), |_, _: &AuthState| {});

	state
}

/// Returns a funciton that can be used to set the auth token
/// Make it directly take an Option<String>
pub fn set_auth_token() -> SignalSetter<Option<String>> {
	let state = use_context::<RwSignal<AuthState>>().expect("State Needs to be provided");
	let (_, set_state) = create_slice(
		state,
		|_| {},
		|state: &mut AuthState, t| {
			if let Some(token) = t {
				*state = AuthState::LoggedIn { auth_token: token }
			} else {
				*state = AuthState::LoggedOut
			}
		},
	);

	set_state
}
