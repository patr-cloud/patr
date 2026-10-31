//! Tests for the common runner components, including the executor.
//!
//! These tests make sure that the runner supervisor and executor can be started
//! and can run a simple workflow.

mod mock_executor;
mod setup;
mod utils;

mod actor_lifecycle;
mod config;
mod edge_cases;
mod managed_mode;
mod managed_server;
mod schema;
mod status_reconciliation;
mod supervision;
mod volumes;

mod prelude {
	pub use std::time::Duration;

	pub use common::prelude::*;
	pub use models::api::workspace::deployment::*;

	pub use crate::{mock_executor::*, setup::*, utils::*};
}
