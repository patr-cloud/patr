use crate::{
	pages::{CreateDatabase, DatabaseDashboard, DatabasePage, ManageDatabase},
	prelude::*,
};

::macros::declare_app_route! {
	/// Route for Databases Dashboard Page
	DatabaseDashboard,
	"/database",
	requires_login = true,
	query = {
		/// Common Query Parameters
		#[serde(flatten)]
		pub common: CommonQueryParams,
	}
}

::macros::declare_app_route! {
	/// Route for Databases Create Page
	CreateDatabase,
	"/database/create",
	requires_login = true,
}

::macros::declare_app_route! {
	/// Route for Databases Details Page
	ManageDatabase,
	"/database/:database_id" {
		/// The id of the database
		pub database_id: Uuid,
	},
	requires_login = true,
}

/// The list of routes for the database stuff
#[component(transparent)]
pub fn DatabaseRoutes() -> impl IntoView {
	view! {
		<AppRoute<DatabaseDashboardRoute, _, _> view={move |_query, _params| DatabasePage}>
			<AppRoute<CreateDatabaseRoute, _, _> view={move |_query, _params| CreateDatabase} />
			<AppRoute<ManageDatabaseRoute, _, _> view={move |_query, _params| ManageDatabase} />
			<Route path={AppRoutes::Empty} view={DatabaseDashboard} />
		</AppRoute<DatabaseDashboardRoute, _, _>>
	}
}
