use models::api::auth::*;

use crate::{
	pages::{DomainsDashboard, LoginPage, ManagedUrlDashboard, *},
	prelude::*,
};

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
					<Skeleton
						class={"full-width".to_owned()}
						enable_full_height={true}
						enable_full_width={true}
					/>
				</header>

				<ManageDeployments />
			</main>
		</div>
	}
}

#[component]
pub fn App() -> impl IntoView {
	view! {
		<LoggedInPage />
	}
}
