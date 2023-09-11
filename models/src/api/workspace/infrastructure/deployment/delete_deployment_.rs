use crate::prelude::*;

macros::declare_api_endpoint!(
	// Delete deployment
	DeleteDeployment,
	DELETE "/workspace/:workspace_id/infrastructure/deployment/:deployment_id",
    path = {
        pub workspace_id: Uuid,
        pub deployment_id: Uuid
    },
	request = {
	    pub hard_delete: bool,
	}
);
