use crate::prelude::*;

macros::declare_app_route! {
	/// The route that shows a single deployment in a workspace
	DeploymentDetails,
	"/deployment/{deployment_id}" {
		/// The ID of the deployment to show
		pub deployment_id: Uuid,
	},
	requires_login = true,
	query = {
		/// The workspace ID to switch to when creating the deployment
		#[serde(skip_serializing_if = "Option::is_none")]
		pub workspace_id: Option<Uuid>,
	},
}
