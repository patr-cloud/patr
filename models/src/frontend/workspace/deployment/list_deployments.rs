use crate::prelude::*;

macros::declare_app_route! {
	/// The route that lists all deployments in a workspace
	ListDeployments,
	"/deployment",
	requires_login = true,
	query = {
		/// The workspace ID to switch to when listing deployments
		#[serde(skip_serializing_if = "Option::is_none")]
		pub workspace_id: Option<Uuid>,
	},
}
