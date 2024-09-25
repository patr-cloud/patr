use models::utils::Uuid;

use crate::CommonQueryParams;

macros::declare_app_route! {
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

macros::declare_app_route! {
	/// Route for Runners Create Page
	CreateRunner,
	"/runner/create",
	requires_login = true,
	query = {}
}

macros::declare_app_route! {
	/// Route for Runners Details Page
	ManageRunner,
	"/runner/:runner_id" {
		/// The id of the runner
		pub runner_id: Uuid,
	},
	requires_login = true,
	query = {}
}
