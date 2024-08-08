#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]
#![feature(impl_trait_in_assoc_type)]

//! Common utilities for the runner. This library contains all the things you
//! will need to make a runner. All it needs are the implementations of how the
//! runner handles the resources (such as deployments, databases, etc).

/// The client for the Patr API to get runner data for a given workspace.
mod client;
/// The configuration for the runner.
mod config;
/// A utility that returns a value after a delay.
mod delayed_future;
/// The executor module contains the trait that the runner needs to implement
/// to run the resources.
mod executor;
/// Extensions traits for the `Either` type.
mod ext_traits;
/// All the runner related structs and functions.
mod runner;

/// The prelude module contains all the things you need to import to get
/// started with the runner.
pub mod prelude {
	pub use macros::{query, version};
	pub use models::prelude::*;

	pub(crate) use crate::ext_traits::EitherExt;
	pub use crate::{config::*, executor::RunnerExecutor, runner::Runner};

	/// The client module to interface with the Patr API
	pub mod client {
		pub use crate::client::*;
	}
}
