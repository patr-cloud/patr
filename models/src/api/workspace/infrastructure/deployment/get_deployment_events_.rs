use crate::prelude::*;

macros::declare_api_endpoint!(
    // Get events for a deployment
	GetDeploymentEvents,
	GET "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/events",
    path = {
        pub workspace_id: Uuid,
        pub deployment_id: Uuid
    },
    response = {
        pub logs: Vec<WorkspaceAuditLog>,
    }
);
