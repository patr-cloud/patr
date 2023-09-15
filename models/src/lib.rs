#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::missing_docs_in_private_items)]
#![feature(return_position_impl_trait_in_trait)]

//! This crate contains all the DTOs and common models used in the API across
//! the frontend, backend, the CLI, and the controller.

/// All the route definitions
pub mod api;
/// All the CI structs and formats
pub mod ci;
/// All utilities commonly used by clients, including a fetch function, and a
/// [`tower::Layer`] that refreshes access tokens and retries requests in case
/// in case the token expires.
pub mod client;
/// Any data that is sent to or from cloudflare (mostly KV)
pub mod cloudflare;
/// All infrastructure as code related structs and formats
pub mod iaac;
/// Utility functions and structs
pub mod utils;

/// The prelude module contains all the commonly used types and traits that are
/// used across the crate. This is mostly used to avoid having to import a lot
/// of things from different modules.
pub mod prelude {
	pub use crate::{
		utils::{LoginId, OneOrMore, Paginated, TotalCountHeader, Uuid},
		ApiEndpoint,
		ApiRequest,
		ApiSuccessResponse,
		ErrorType,
	};
}

mod private {
	pub trait Sealed {}
}

mod endpoint;
mod error;
mod request;
mod response;
mod user_data;

pub use self::{endpoint::*, error::*, request::*, response::*, user_data::*};
