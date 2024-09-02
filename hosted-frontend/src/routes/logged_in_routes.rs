use leptos_router::{Outlet, ProtectedRoute};

use crate::{prelude::*, utils::AuthState};

/// The Outer View for Logged In Route
#[component]
pub fn LoggedInRoutesView() -> impl IntoView {
	let (state, _) = AuthState::load();

	move || match state.get() {
		AuthState::LoggedOut => view! {
			<PageContainer class="bg-image">
				<Outlet />
			</PageContainer>
		}
		.into_view(),
		AuthState::LoggedIn { .. } => view! {
			<div class="fr-fs-fs full-width full-height bg-secondary">
				<Outlet />
			</div>
		}
		.into_view(),
	}
}

/// Contains all the routes for when the user is logged in
#[component(transparent)]
pub fn LoggedInRoutesComponent() -> impl IntoView {
	let (state, _) = AuthState::load();

	view! {
		<ProtectedRoute
			path={AppRoutes::Empty}
			view={LoggedInRoutesView}
			redirect_path={AppRoutes::LoggedOutRoute(LoggedOutRoute::Login)}
			condition={move || state.get().is_logged_in()}
		>
			<WorkspacedRoutes />
			<NotWorkspacedRoutes />
		</ProtectedRoute>
	}
}
