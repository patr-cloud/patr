/// This module contains all the utilities used for parsing a request and using
/// it in the [`crate::ApiEndpoint`] request struct.
mod axum_request;
/// This module contains all the utilities used for parsing a response and using
/// it in the [`crate::ApiEndpoint`] response struct.
mod axum_response;
/// A set of constant booleans that are used to ensure that the values are
/// forced to be either true or false.
mod bools;
/// A set of utilities to parse headers from a request, ensure that certain
/// headers are present in a struct as well as provide what headers are required
/// for an endpoint.
mod header_utils;
/// A set of middlewares that are used by the API to perform certain tasks, like
/// authentication, audit logging, etc.
mod middlewares;
/// Represents a value that can be either one or many. This is used to represent
/// a value that can be either a single value or a list of values, such as
/// audience in a JWT, a dependency string in a CI yaml file, etc.
mod one_or_many;
/// A set of utilities to parse a paginated response from the API. A paginated
/// request enforces a response header to be present, which provides the total
/// number of items in the response.
mod paginated;
/// A set of utilities to work with tuples. This is mostly used in adding a
/// required response header for [`paginated`][super::paginated] responses.
mod tuple_utils;
/// A wrapper around [`uuid::Uuid`] that implements [`serde::Serialize`] and
/// [`serde::Deserialize`] in a particular format. This is used to ensure that
/// the UUIDs are always serialized and deserialized in the same format.
mod uuid;
/// Websocket utilities, providing a request that can be used to upgrade an HTTP
/// request to a websocket connection.
mod websocket;

pub use self::{
	axum_request::*,
	axum_response::*,
	bools::*,
	header_utils::*,
	middlewares::*,
	one_or_many::*,
	paginated::*,
	tuple_utils::*,
	uuid::*,
	websocket::*,
};

/// All the constants used in the application.
/// Constants are used to avoid hardcoding values, since that might introduce
/// typos.
pub mod constants {
	/// Base URL for the API
	pub const API_BASE_URL: &str = "https://api.patr.cloud";
}
