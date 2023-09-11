use crate::prelude::*;

macros::declare_api_endpoint!(
    // Update a deployment
    UpdateDeployment,
	PATCH "/workspace/:workspace_id/infrastructure/deployment/:deployment_id",
    path = {
        pub workspace_id: Uuid,
        pub deployment_id: Uuid
    },
    request = {
        pub name: Option<String>,
        pub machine_type: Option<Uuid>,
        pub deploy_on_push: Option<bool>,
        pub min_horizontal_scale: Option<u16>,
        pub max_horizontal_scale: Option<u16>,
        pub ports: Option<BTreeMap<StringifiedU16, ExposedPortType>>,
        pub environment_variables:
            Option<BTreeMap<String, EnvironmentVariableValue>>,
        pub startup_probe: Option<DeploymentProbe>,
        pub liveness_probe: Option<DeploymentProbe>,
        pub config_mounts: Option<BTreeMap<String, Base64String>>,
        pub volumes: Option<BTreeMap<String, DeploymentVolume>>,
    }
);
