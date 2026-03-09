//! The Docker runner is a service that runs on a machine and listens for
//! incoming WebSocket connections from the Patr API. The runner is responsible
//! for creating, updating, and deleting deployments in the given runner.

/// Grafana Alloy log collector service management
pub(crate) mod alloy;
/// The configuration of the Docker runner
pub(crate) mod config;
/// All deployment related stuff goes here
pub(crate) mod deployment;
/// The module to handle ingress and routing
pub(crate) mod ingress;
/// The core runner implementation that interfaces between Patr and Docker
pub(crate) mod runner;
/// Any additional utilities that are commonly used in the runner
pub(crate) mod utils;

/// All commonly used imports for the Docker runner, wrapped neatly in a
/// prelude.
pub mod prelude {
	pub use common::prelude::*;

	pub use crate::{config::DockerSettings, runner::DockerRunner, utils::constants};
	pub(crate) use crate::{alloy, deployment, ingress};
}
