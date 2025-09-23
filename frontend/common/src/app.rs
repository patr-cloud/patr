use leptos_router::{components::*, path};

use crate::prelude::*;
/// The Entry Point for the whole app, here's where routers and all are defined
#[component]
pub fn App(
	/// The [`AppType`] of the application. This is used to determine which
	/// app to run.
	app_type: AppType,
) -> impl IntoView {
	provide_context(app_type);
	view! {
		<div id="root">
		<Router>
			<Routes fallback=||view! { <p>"NOT_FOUND"</p> }>
				<Route path=path!("/infrastructure/runner") view=RunnerDashboard/>
				<Route path=path!("/login") view=LoginPage/>
			</Routes>
		</Router>
	</div>
	}
}
