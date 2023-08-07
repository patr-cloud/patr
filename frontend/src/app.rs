use leptos_router::{
	use_location,
	Outlet,
	ProtectedRoute,
	Redirect,
	Route,
	Router,
	Routes,
};
use leptos_use::storage::use_local_storage;

use crate::prelude::*;

#[component]
pub fn App(cx: Scope) -> impl IntoView {
	let (state, set_state, _) =
		use_local_storage(cx, "app-storage", AppStorage::default());
	provide_context(cx, state);
	provide_context(cx, set_state);

	view! { cx,
		<Router>
			<Routes>
				// Logged out routes
				<ProtectedRoute
					path=AppRoute::Empty
					// If not logged out (as in if logged in), redirect to home
					redirect_path=AppRoute::LoggedInRoutes(LoggedInRoutes::Home)
					condition=move |_| !is_logged_in(cx)
					view=|cx| view! {cx,
						<div class="fc-ct-ct bg-page bg-onboard">
							<Outlet />
						</div>
					}
					>
					<Route path=LoggedOutRoutes::Login view=Login />
					<Route path=LoggedOutRoutes::SignUp view=SignUp />
					<Route path="*" view=|cx| {
						let location = use_location(cx);
						log::info!("location: {}", location.pathname.get());
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
						let path = format!(
							"{}?{}",
							LoggedOutRoutes::Login,
							serde_urlencoded::to_string(&[("to", to)]).unwrap()
						);
						view! { cx,
							<Redirect path=path />
						}
					} />
				</ProtectedRoute>

				// Logged in routes
				<ProtectedRoute
					path=AppRoute::Empty
					// If logged out, redirect to login
					redirect_path=AppRoute::LoggedOutRoutes(LoggedOutRoutes::Login)
					condition=move |_| is_logged_in(cx)
					view=|cx| view! {cx,
						<PageContainer>
							<Outlet />
						</PageContainer>
					}
					>
					<Route path=LoggedInRoutes::Home view=|_| () />

					<Route path="*" view=|cx| {
						view! { cx,
							<Redirect path=LoggedInRoutes::Home />
						}
					} />
				</ProtectedRoute>
			</Routes>
		</Router>
	}
}

fn is_logged_in(cx: Scope) -> bool {
	let state = expect_context::<Signal<AppStorage>>(cx);
	state.get().is_logged_in()
}
