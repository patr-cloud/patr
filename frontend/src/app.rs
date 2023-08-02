use leptos_router::{Outlet, ProtectedRoute, Redirect, Route, Router, Routes};
use leptos_use::storage::use_storage;

use crate::prelude::*;

#[component]
pub fn App(cx: Scope) -> impl IntoView {
	let (state, set_state, _) =
		use_storage(cx, "app-storage", AppStorage::default());

	view! { cx,
		<Router>
			<Routes>
				// Logged out routes
				<ProtectedRoute
					path=AppRoute::Empty
					// If not logged out (as in if logged in), redirect to home
					redirect_path=AppRoute::LoggedInRoutes(LoggedInRoutes::Home)
					condition=move |_| !state.get().is_logged_in()
					view=|cx| view! {cx,
						<div class="fc-ct-ct bg-page bg-onboard">
							<Outlet />
						</div>
					}
					>
					<Route path=LoggedOutRoutes::Login view=Login />
					<Route path=LoggedOutRoutes::SignUp view=SignUp />
					<Route path="*" view=|cx| {
						view! { cx,
							<Redirect path=LoggedOutRoutes::Login />
						}
					} />
				</ProtectedRoute>
			// {move || {
			// 	if state.get().is_logged_in() {
			// 		view! { cx,
			// 			<Route path="" view=move |_| {
			// 				view! { cx,
			// 					<Redirect path="logout" />
			// 				}
			// 			} />
			// 			<Route path="logout" view=move |_| {
			// 				view! { cx,
			// 					<Redirect path="home" />
			// 				}
			// 			} />
			// 		}
			// 	} else {
			// 		log::error!("App state out: {:?}", state.get());
			// 		view! { cx,
			// 		}
			// 	}
			// }}
			</Routes>
		</Router>
	}
}

fn is_logged_in(cx: Scope) -> bool {
	let state = expect_context::<AppStorage>(cx);
	state.is_logged_in()
}

fn is_logged_out(cx: Scope) -> bool {
	!is_logged_in(cx)
}
