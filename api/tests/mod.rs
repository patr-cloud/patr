#![allow(missing_docs, clippy::missing_docs_in_private_items)]

//! This module contains tests for the API. Each test should be in a separate
//! file, and the files should be organized in a way that makes it easy to find
//! and run specific tests.

mod setup;
mod utils;

/// All tests related to the API are in this module.
mod api;
/// All tests related to the loki.patr.cloud log proxy are in this module.
mod loki;
/// All tests related to the OCI registry are in this module.
mod registry;

/// Convenience re-exports for all tests.
pub mod prelude {
	pub use std::str::FromStr as _;

	pub use api::{
		prelude::*,
		routes::registry_patr_cloud::{
			endpoint::RegistryEndpoint,
			request::RegistryUnprocessedApiRequest,
		},
	};
	pub use axum::body::Body;
	pub use http::StatusCode;
	pub use models::{
		ApiRequest,
		ApiSuccessResponseBody,
		utils::{Headers, OptionalHeader},
	};

	pub use crate::{setup::*, utils::*};
}
