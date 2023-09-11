use crate::prelude::*;

macros::declare_api_endpoint!(
    //List all machine types for deployment
    ListAllDeploymentMachineTypes,
    GET "/workspace/:workspace_id/infrastructure/machine-type",
    path = {
        pub workspace_id: Uuid
    },
    response = {
        pub machine_types: Vec<DeploymentMachineType>
    }
);