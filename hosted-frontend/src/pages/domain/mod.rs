use leptos_router::*;

use crate::prelude::*;

/// The Routes for the Domain Configuration
#[component(transparent)]
pub fn DomainConfigurationRoutes() -> impl IntoView {
	view! {
		<Route path={AppRoutes::Empty} view={|| view! { <Outlet /> }}>
			<Route path={LoggedInRoute::ManagedUrl} view={ManagedUrlPage}>
				<Route path="create" view={|| view! { <div>"create"</div> }} />
				<Route path={AppRoutes::Empty} view={UrlDashboard} />
			</Route>
			<Route path={LoggedInRoute::Domain} view={DomainsDashboard} />
		</Route>
	}
}

mod domains;
mod managed_url;

pub use self::{domains::*, managed_url::*};
