use leptos_router::*;

use crate::prelude::*;

#[component(transparent)]
pub fn DomainConfigurationRoutes() -> impl IntoView {
	view! {
		<Route path={AppRoutes::Empty} view={|| view! { <Outlet/> }}>
			<Route path={LoggedInRoute::ManagedUrl} view={ManagedUrlDashboard}/>
			<Route path={LoggedInRoute::Domain} view={DomainsDashboard}/>
		</Route>
	}
}

mod domains;
mod managed_url;

pub use self::{domains::*, managed_url::*};
