use leptos_router::{use_location, Outlet, ProtectedRoute, Redirect, Route, Router, Routes};
use models::api::auth::*;

use crate::{pages::*, prelude::*};

#[allow(async_fn_in_trait)] // WIP
pub trait AppAPIs {
	async fn login(
		request: ApiRequest<LoginRequest>,
	) -> Result<AppResponse<LoginRequest>, ServerFnError<ErrorType>>;
}
#[component]
fn LoggedInPage() -> impl IntoView {
	view! {
		<div class="fr-fs-fs full-width full-height bg-secondary">
			<Sidebar />
			<main class="fc-fs-ct full-width px-lg">
				// This is a temporary empty div for the header
				<header style="width: 100%; min-height: 5rem;">
				</header>

				<ManageProfile />
			</main>
		</div>
	}
}

#[component]
pub fn App() -> impl IntoView {
	view! {
		<Router>
			<Routes>
				<AppRoute
					route=LoginRoute {}
					view=|_| LoginPage()
				/>
			</Routes>
		</Router>
	}
}
