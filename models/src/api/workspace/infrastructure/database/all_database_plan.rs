use crate::{
    prelude::*,
    utils::BearerToken
};
use serde::{Deserialize, Serialize};

/// Information of all the different database plans currently supported
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatabasePlan {
    /// The number of CPU nodes
	pub cpu_count: i32,
    /// The number of memory nodes
	pub memory_count: i32,
    /// The size of the volume
	pub volume: i32,
}

macros::declare_api_endpoint!(
    /// Route to get database information
    AllDatabasePlan,
    GET "/workspace/infrastructure/database/plan",
    request_headers = {
        /// Token used to authorize user
        pub authorization: BearerToken
    },
    response = {
        /// List of database plans containing:
        /// cpu_count: The number of CPU nodes
        /// memory_count: The number of memory nodes
        /// volume: The size of the volume
        pub plans: Vec<WithId<DatabasePlan>>
    }
);