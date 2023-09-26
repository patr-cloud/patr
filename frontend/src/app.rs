use leptos_router::{
	use_location,
	Outlet,
	ProtectedRoute,
	Redirect,
	Route,
	Router,
	Routes,
};

use crate::prelude::*;

#[component]
pub fn App(cx: Scope) -> impl IntoView {
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
					// <Route path="/*logged_in_route" view=|cx| {
					// 	let location = use_location(cx);
					// 	info!("location: {}", location.pathname.get());
					// 	let to = if location.search.get().is_empty() {
					// 		format!(
					// 			"{}{}",
					// 			location.pathname.get(),
					// 			location.hash.get(),
					// 		)
					// 	} else {
					// 		format!(
					// 			"{}{}{}",
					// 			location.pathname.get(),
					// 			location.search.get(),
					// 			location.hash.get()
					// 		)
					// 	};
					// 	let path = if to.is_empty() {
					// 		LoggedOutRoutes::Login.to_string()
					// 	} else {
					// 		format!(
					// 			"{}?{}",
					// 			LoggedOutRoutes::Login,
					// 			serde_urlencoded::to_string([("to", to)]).unwrap()
					// 		)
					// 	};
					// 	view! { cx,
					// 		<Redirect path=path />
					// 	}
					// } />
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

					// <Route path="/*logged_out_route" view=|cx| {
					// 	view! { cx,
					// 		<Redirect path=LoggedInRoutes::Home />
					// 	}
					// } />
				</ProtectedRoute>
			</Routes>
		</Router>
	}
}

/// Returns a boolean if the user is logged in or not
fn is_logged_in(cx: Scope) -> bool {
	// let state = expect_context::<Signal<AppStorage>>(cx);
	// state.get().is_logged_in()
	false
}
