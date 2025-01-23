use leptos_router::components::{Outlet, ProtectedParentRoute};

use crate::{prelude::*, utils::AuthState};

/// The Outer View for Logged In Route
#[component]
pub fn LoggedInRoutesView() -> impl IntoView {
	let (state, _) = AuthState::load();

	move || match state.get() {
		AuthState::LoggedOut => Either::Left(view! {
			<PageContainer class="bg-image">
				<Outlet />
			</PageContainer>
		}),
		AuthState::LoggedIn { .. } => Either::Right(view! {
			<div class="fr-fs-fs full-width full-height bg-secondary">
				<Outlet />
			</div>
		}),
	}
}

/// Contains all the routes for when the user is logged in
#[component(transparent)]
pub fn LoggedInRoutesComponent() -> impl IntoView {
	let (state, _) = AuthState::load();

	view! {
		<ProtectedParentRoute
			path={AppRoutes::Empty}
			view={LoggedInRoutesView}
			redirect_path={|| AppRoutes::LoggedOutRoute(LoggedOutRoute::Login)}
			condition={move || Some(state.get().is_logged_in())}
		>
			<WorkspacedRoutes />
			<NotWorkspacedRoutes />
		</ProtectedParentRoute>
	}
}
