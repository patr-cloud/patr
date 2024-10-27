use models::utils::Uuid;

use crate::{
	pages::{CreateRunner, ManageRunner, RunnerDashboard, RunnerPage},
	prelude::*,
	CommonQueryParams,
};

::macros::declare_app_route! {
	/// Route for Runners Dashboard Page
	RunnerDashboard,
	"/runner",
	requires_login = true,
	query = {
		/// Common Query Parameters
		#[serde(flatten)]
		pub common: CommonQueryParams,
	}
}

::macros::declare_app_route! {
	/// Route for Runners Create Page
	CreateRunner,
	"/runner/create",
	requires_login = true,

}

::macros::declare_app_route! {
	/// Route for Runners Details Page
	ManageRunner,
	"/runner/:runner_id" {
		/// The id of the runner
		pub runner_id: Uuid,
	},
	requires_login = true,

}

/// The Routes for the Runner Page
#[component(transparent)]
pub fn RunnerRoutes() -> impl IntoView {
	view! {
		<>
			<AppRoute<RunnerDashboardRoute, _, _> view={move |_query, _params| RunnerPage} />
			<AppRoute<CreateRunnerRoute, _, _> view={move |_query, _params| CreateRunner} />
			<AppRoute<ManageRunnerRoute, _, _> view={move |_query, _params| ManageRunner} />
			<AppRoute<RunnerDashboardRoute, _, _> view={move |_, _| RunnerDashboard} />
		</>
	}
}
