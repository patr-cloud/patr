use crate::prelude::*;

macros::declare_app_route! {
	/// The route that shows the deployment creation form
	CreateDeployment,
	"/deployment/new",
	requires_login = true,
	query = {
		/// The workspace ID to switch to when creating the deployment
		#[serde(skip_serializing_if = "Option::is_none")]
		pub workspace_id: Option<Uuid>,
	},
}
