use serde::{Deserialize, Serialize};

/// This module contains all the utilities used for parsing a request and using
/// it in the [`crate::ApiEndpoint`] request struct.
mod axum_request;
/// This module contains all the utilities used for parsing a response and using
/// it in the [`crate::ApiEndpoint`] response struct.
mod axum_response;
/// Contains the [`Base64String`] struct, which is used to represent a string
/// that is encoded in base64. This is used to ensure that the base64 string is
/// always serialized and deserialized correctly.
mod base64string;
/// A set of constant booleans that are used to ensure that the values are
/// forced to be either true or false.
mod bools;
/// Represents a location on the planet. This is used to represent the location
/// of a user, a login, etc. Basically just a latitude and longitude.
mod geo_location;
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
/// A helper type that serializes and deserializes u16 values as strings. This
/// is used for using u16 values as keys in a JSON object.
mod stringified_u16;
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
	api_list_utils::*,
	axum_request::*,
	axum_response::*,
	base64string::*,
	bools::*,
	geo_location::*,
	header_utils::*,
	middlewares::*,
	one_or_many::*,
	paginated::*,
	stringified_u16::*,
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

	/// Patr's container registry URL
	pub const CONTAINER_REGISTRY_URL: &str = "registry.patr.cloud";

	/// A NodeID for Uuid v1.
	/// Spells "*Patr*" in bytes
	pub const UUID_NODE_ID: [u8; 6] = [42, 80, 97, 116, 114, 42];
}

/// Ordering of the list for paginated requests
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ListOrder {
	/// Ascending order
	Ascending,
	/// Descending order
	#[default]
	Descending,
}
