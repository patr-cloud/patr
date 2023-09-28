use leptos_router::{Outlet, ProtectedRoute, Route, Router, Routes, use_location, Redirect};

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
					condition=|cx| dbg!(!is_logged_in(cx))
					view=|cx| view! {cx,
						<div class="fc-ct-ct bg-page bg-onboard">
							<Outlet />
						</div>
					}
					>
					<Route path=LoggedOutRoutes::Login view=Login />
					<Route path=LoggedOutRoutes::SignUp view=SignUp />
				</ProtectedRoute>

				// Logged in routes
				<ProtectedRoute
					path=AppRoute::Empty
					// If logged out, redirect to login
					redirect_path=AppRoute::LoggedOutRoutes(LoggedOutRoutes::Login)
					condition=|cx| dbg!(is_logged_in(cx))
					view=|cx| view! {cx,
						<PageContainer>
							<Outlet />
						</PageContainer>
					}
					>
					<Route path="/".to_string() view=|_| () /> // TODO show root page
					<Route path=LoggedInRoutes::Home view=|_| () /> // TODO show home page
				</ProtectedRoute>

				// <Route path="/*other" view=|cx| {
				// 	if is_logged_in(cx) {
				// 		view! { cx,
				// 			<Redirect path=LoggedInRoutes::Home />
				// 		}
				// 	} else {
				// 		let location = use_location(cx);
				// 		info!("location: {}", location.pathname.get());
				// 		let to = if location.search.get().is_empty() {
				// 			format!(
				// 				"{}{}",
				// 				location.pathname.get(),
				// 				location.hash.get(),
				// 			)
				// 		} else {
				// 			format!(
				// 				"{}{}{}",
				// 				location.pathname.get(),
				// 				location.search.get(),
				// 				location.hash.get()
				// 			)
				// 		};
				// 		let path = if to.is_empty() {
				// 			LoggedOutRoutes::Login.to_string()
				// 		} else {
				// 			format!(
				// 				"{}?{}",
				// 				LoggedOutRoutes::Login,
				// 				serde_urlencoded::to_string([("to", to)]).unwrap()
				// 			)
				// 		};
				// 		view! { cx,
				// 			<Redirect path=path />
				// 		}
				// 	}
				// } />
			</Routes>
		</Router>
	}
}

/// Returns a boolean if the user is logged in or not
fn is_logged_in(_cx: Scope) -> bool {
	// let state = expect_context::<Signal<AppStorage>>(cx);
	// state.get().is_logged_in()
	dbg!(false)
}
