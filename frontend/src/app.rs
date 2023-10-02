use leptos_router::{use_location, Outlet, ProtectedRoute, Redirect, Route, Router, Routes};

use crate::prelude::*;

#[component]
pub fn App() -> impl IntoView {
	view! {
		<Router>
			<Routes>
				// Logged out routes
				<ProtectedRoute
					path=AppRoute::Empty
					// If not logged out (as in if logged in), redirect to home
					redirect_path=AppRoute::LoggedInRoute(LoggedInRoute::Home)
					condition=|| !is_logged_in()
					view=|| view! {
						<div class="fc-ct-ct bg-page bg-onboard">
							<Outlet />
						</div>
					}
					>
					<Route path=LoggedOutRoute::Login view=Login />
					<Route path=LoggedOutRoute::SignUp view=SignUp />
				</ProtectedRoute>

				// Logged in routes
				<ProtectedRoute
					path=AppRoute::Empty
					// If logged out, redirect to login
					redirect_path=AppRoute::LoggedOutRoute(LoggedOutRoute::Login)
					condition=|| is_logged_in()
					view=|| view! {
						<PageContainer>
							<Outlet />
						</PageContainer>
					}
					>
					<Route path=LoggedInRoute::Home view=|| () /> // TODO show home page
				</ProtectedRoute>

				<Route path="/*other" view=|| {
					if is_logged_in() {
						NotFound().into_view()
					} else {
						let location = use_location();
						info!("location: {}", location.pathname.get());
						let to = if location.search.get().is_empty() {
							format!(
								"{}{}",
								location.pathname.get(),
								location.hash.get(),
							)
						} else {
							format!(
								"{}{}{}",
								location.pathname.get(),
								location.search.get(),
								location.hash.get()
							)
						};
						let path = if to.is_empty() {
							LoggedOutRoute::Login.to_string()
						} else {
							format!(
								"{}?{}",
								LoggedOutRoute::Login,
								serde_urlencoded::to_string([("to", to)]).unwrap()
							)
						};
						view! {
							<Redirect path=path />
						}
					}
				} />
			</Routes>
		</Router>
	}
}

/// Returns a boolean if the user is logged in or not
fn is_logged_in() -> bool {
	// let state = expect_context::<Signal<AppStorage>>();
	// state.get().is_logged_in()
	true
}
