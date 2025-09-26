use crate::prelude::*;

/// The Entry Point for the whole app, here's where routers and all are defined
#[component]
pub fn App(
	/// The [`AppType`] of the application. This is used to determine which
	/// app to run.
	app_type: AppType,
) -> impl IntoView {
	provide_context(app_type);

	let auth_state = AuthState::load();

	move || {
		if auth_state.get().is_logged_in() {
			Either::Left(view! {
				<LoggedInContent />
			})
		} else {
			Either::Right(view! {
				<LoggedOutContent />
			})
		}
	}
}
