use crate::prelude::*;

macros::declare_api_endpoint!(
    // Create deployment
	CreateDeployment,
	POST "/workspace/:workspace_id/infrastructure/deployment",
    path = {
        pub workspace_id: Uuid,
    },
	request = {
		pub name: String,
        pub registry: DeploymentRegistry,
        pub image_tag: String,
        pub region: Uuid,
        pub machine_type: Uuid,
        pub running_details: DeploymentRunningDetails,
        pub deploy_on_create: bool,
	},
	response = {
			pub id: Uuid,
	}
);
