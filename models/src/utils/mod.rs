use std::borrow::Cow;

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

/// The function to validate if a password has:
/// - A minimum of 8 characters
/// - Must contain atleast one digit
/// - One uppercase letter
/// - One lowercase letter
/// - One special character (!@#$%^&*?)
pub fn validate_password(value: Cow<'_, str>) -> Result<Cow<'_, str>, preprocess::Error> {
	use preprocess::Error;

	let (has_digit, has_uppercase, has_lowercase, has_special) = value.chars().fold(
		(false, false, false, false),
		|(has_digit, has_uppercase, has_lowercase, has_special), value| {
			(
				has_digit || value.is_ascii_digit(),
				has_uppercase || value.is_ascii_uppercase(),
				has_lowercase || value.is_ascii_lowercase(),
				has_special ||
					matches!(
						value,
						'@' | '!' |
							'#' | '$' | '%' | '^' | '&' |
							'*' | '?' | '/' | '\\' |
							'|' | '~' | '`' | '.' | ',' |
							';' | ':' | '<' | '>' | '[' |
							']' | '{' | '}'
					),
			)
		},
	);

	if !has_digit {
		return Err(Error::new("Password must contain at least one digit"));
	}

	if !has_lowercase {
		return Err(Error::new("Password must contain at least one lowercase"));
	}

	if !has_uppercase {
		return Err(Error::new("Password must contain at least one uppercase"));
	}

	if !has_special {
		return Err(Error::new(
			"Password must contain at least one special character",
		));
	}

	Ok(value)
}

/// All the constants used in the application.
/// Constants are used to avoid hardcoding values, since that might introduce
/// typos.
pub mod constants {
	/// Base URL for the API
	pub const API_BASE_URL: &str = if cfg!(debug_assertions) {
		"http://localhost:3000"
	} else {
		"https://api.patr.cloud"
	};

	/// Patr's container registry URL
	pub const CONTAINER_REGISTRY_URL: &str = "registry.patr.cloud";

	/// A NodeID for Uuid v1.
	/// Spells "*Patr*" in bytes
	pub const UUID_NODE_ID: [u8; 6] = [42, 80, 97, 116, 114, 42];

	/// The regular expression used to validate a username.
	///
	/// The username must start with an alphanumeric character or an underscore,
	/// and end with an alphanumeric character. The username can contain
	/// alphanumeric characters, underscores, dots, and hyphens.
	pub const USERNAME_VALIDITY_REGEX: &str =
		macros::verify_regex!(r"^[a-z0-9_][a-z0-9_\.\-]*[a-z0-9_]$");

	/// Regex to validate The Country Code of the phone number.
	///
	/// The country code is a 2-letter code that represents the country of the
	/// phone number. The country code must be in the format `US`, `IN`, `UK`,
	/// etc.
	pub const PHONE_NUMBER_COUNTRY_CODE_REGEX: &str = macros::verify_regex!(r"^[A-Z][A-Z]$");

	/// The Regex to validate the phone number. The phone number must be in the
	/// standard 10-digit number format. The number must be in the format `(123)
	/// 456 7890`, `123-456-7890, 1234567890, 123.456.7890`,
	pub const PHONE_NUMBER_REGEX: &str =
		macros::verify_regex!(r"^\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}$");

	/// The Regex to validate OTP of the user. The OTP must be a 6-digit number.
	/// The OTP can be of the format `123456` or `123-456`.
	pub const OTP_VERIFICATION_TOKEN_REGEX: &str = macros::verify_regex!(r"^(\d{3}\-?\d{3})$");

	/// The Regex to validate a resource name (e.g. deployment name, etc.)
	/// Matches a string that is between 4 and 255 characters long and can have
	/// digits, letters, hyphens, underscores, spaces and dots.
	pub const RESOURCE_NAME_REGEX: &str = macros::verify_regex!(r"^[a-zA-Z0-9\-_ \.]{4,255}$");

	/// The Regex to validate a DNS record name.
	///
	/// The DNS record name must be in the format `@`, `www`, `subdomain`, etc.
	/// The DNS record name can have alphanumeric characters and hyphens, but
	/// must not start or end with a hyphen.
	pub const DNS_RECORD_NAME_REGEX: &str = macros::verify_regex!(
		r"^((([a-z0-9].)([a-z0-9\-]*){0,63}([a-z0-9].).)(\.([a-z0-9].)([a-z0-9\-_]*){0,63}([a-z0-9]*)))|\@$"
	);
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
