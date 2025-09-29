use codee::string::JsonSerdeCodec;
use leptos_router::{
	components::{Outlet, ProtectedParentRoute, Router, Routes},
	path,
};
use leptos_use::{UseCookieOptions, use_cookie_with_options};

use crate::prelude::*;

/// The Entry Point for the whole app, here's where routers and all are defined
#[component]
pub fn App(
	/// The [`AppType`] of the application. This is used to determine which
	/// app to run.
	app_type: AppType,
) -> impl IntoView {
	provide_context(app_type);

	let auth_state = use_cookie_with_options::<AuthState, JsonSerdeCodec>(
		constants::AUTH_STATE,
		UseCookieOptions::default()
			.http_only(false)
			.secure(if cfg!(debug_assertions) { false } else { true }),
	)
	.0
	.map(|value| value.unwrap_or_default());

	// Provide the auth state to the context
	provide_context(auth_state);

	view! {
		<Router>
			<Routes fallback=NotFoundPage>
				<ProtectedParentRoute
					path=path!("")
					view=LoggedOutHolder
					condition=move || Some(auth_state.get().is_logged_out())
					redirect_path=|| "/"
				>
					<LoggedOutContent />
				</ProtectedParentRoute>
				<ProtectedParentRoute
					path=path!("")
					view=Outlet
					condition=move || Some(auth_state.get().is_logged_in())
					redirect_path=|| "/login"
				>
					<LoggedInContent />
				</ProtectedParentRoute>
			</Routes>
		</Router>
	}
}
