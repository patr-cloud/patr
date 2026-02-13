//! This module contains tests for the API. Each test should be in a separate
//! file, and the files should be organized in a way that makes it easy to find
//! and run specific tests.

/// All tests related to the API are in this module.
mod api;

/// Convenience re-exports for the API tests.
pub mod prelude {
	pub use crate::api::setup;
}
