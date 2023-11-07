use crate::{
	prelude::*,
	utils::{Uuid, BearerToken}, api::workspace::WorkspaceAuditLog,
};

macros::declare_api_endpoint!(
	/// Route to get events of a deployments like pod messages, activities, etc
	GetDeploymentEvent,
	GET "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/events" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The deployment ID to get the event details for
		pub deployment_id: Uuid
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator { 
			extract_resource_id: |req| req.path.deployment_id
		}
	},
	response = {
		/// The event logs of the deployment containing:
		/// id - The event ID
		/// date - The date of the event
		/// ip_address - The IP address of the user
		/// workspace_id - The accessed workspace ID
		/// user_id - The user ID triggering the event
		/// login_id - The session ID triggering the event
		/// resource_id - The resouece ID being accessed
		/// action - The action taken on the resource
		/// request_id - The request ID of the event
		/// metadata - The metadata value
		/// patr_action - Is it patr action or not
		/// request_sucess - Was the request successful or not
		pub logs: Vec<WithId<WorkspaceAuditLog>>,
	}
);
