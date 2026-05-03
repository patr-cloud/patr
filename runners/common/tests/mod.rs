//! Tests for the common runner components, including the executor and server.
//!
//! These tests make sure that the runner supervisor and executor can be started
//! and can run a simple workflow. They also test the HTTP routes exposed by the
//! runner server, which are used by the CLI and API server to interact with the
//! runner.

mod mock_executor;
mod setup;
mod utils;

mod actor_lifecycle;
mod edge_cases;
mod http_routes;
mod managed_mode;
mod managed_server;
mod schema;
mod status_reconciliation;
mod supervision;

mod prelude {
	pub use std::time::Duration;

	pub use common::prelude::*;
	pub use models::api::workspace::deployment::*;

	pub use crate::{mock_executor::*, setup::*, utils::*};
}
