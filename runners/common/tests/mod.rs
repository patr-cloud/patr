//! Tests for the common runner components, including the executor.
//!
//! These tests make sure that the runner supervisor and executor can be started
//! and can run a simple workflow.

mod mock_executor;
mod setup;
mod utils;

mod config;
mod controller;
mod edge_cases;
mod expected_graph;
mod managed_mode;
mod managed_server;
mod reconcile;
mod schema;
mod volumes;

mod prelude {
	pub use std::time::Duration;

	pub use common::prelude::*;
	pub use models::api::workspace::deployment::*;

	pub use crate::{mock_executor::*, setup::*, utils::*};
}
