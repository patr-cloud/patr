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
/// A set of extension traits that provide additional functionality to the
/// [`IaacResource`] type, such as deduplication of resources based on their
/// names.
mod ext_trait;
/// Represents a location on the planet. This is used to represent the location
/// of a user, a login, etc. Basically just a latitude and longitude.
mod geo_location;
/// A set of utilities to parse headers from a request, ensure that certain
/// headers are present in a struct as well as provide what headers are required
/// for an endpoint.
mod headers;
/// Extension traits for iterators.
mod iterator_ext;
/// A set of utilities to parse a query param for the list route API. This route
/// enforces a response header to be present, which provides the total number of
/// items in the response, as well as adding other params like sorting,
/// filtering, etc.
mod list_resource_query;
/// A set of middlewares that are used by the API to perform certain tasks, like
/// authentication, audit logging, etc.
mod middlewares;
/// Represents a value that can be either one or many. This is used to represent
/// a value that can be either a single value or a list of values, such as
/// audience in a JWT, a dependency string in a CI yaml file, etc.
mod one_or_many;
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
	ext_trait::*,
	geo_location::*,
	headers::*,
	iterator_ext::*,
	list_resource_query::*,
	middlewares::*,
	one_or_many::*,
	stringified_u16::*,
	tuple_utils::*,
	uuid::*,
	websocket::*,
};

/// Ordering of the list for paginated requests
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SortOrder {
	/// Ascending order
	Ascending,
	/// Descending order
	#[default]
	Descending,
}

/// A trait that represents a type that can be checked for emptiness.
/// This is used to provide a common interface for types that can be checked
/// for emptiness, for not serializing empty values
pub trait IsEmpty {
	/// Returns true if the value is empty, false otherwise.
	fn is_empty(&self) -> bool;
}

impl IsEmpty for () {
	fn is_empty(&self) -> bool {
		true
	}
}

/// The function to validate if a password has:
/// - A minimum of 8 characters
/// - Must contain atleast one digit
/// - One uppercase letter
/// - One lowercase letter
/// - One special character (!@#$%^&*?)
///
/// # Errors
/// Returns an error if the password does not meet any of the requirements.
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

	/// A `NodeID` for Uuid v1.
	/// Spells "*Patr*" in bytes
	pub const UUID_NODE_ID: [u8; 6] = [42, 80, 97, 116, 114, 42];

	/// The Regex to validate OTP of the user. The OTP must be a 6-digit number.
	/// The OTP can be of the format `123456` or `123-456`.
	pub const OTP_VERIFICATION_TOKEN_REGEX: &str = macros::verify_regex!(r"^(\d{3}\-?\d{3})$");

	/// The Regex to validate a resource name (e.g. deployment name, etc.)
	/// Matches a string that is between 4 and 255 characters long and can have
	/// digits, letters, hyphens, underscores, spaces and dots.
	pub const RESOURCE_NAME_REGEX: &str = macros::verify_regex!(r"^[a-zA-Z0-9\-_ \.]{4,255}$");

	/// The Regex to validate a container registry repository name. Unlike a
	/// generic resource name, this must satisfy the registry's storage
	/// constraint: lowercase alphanumerics in dot/underscore/dash-separated
	/// segments (the shape Docker/OCI accepts). Validating it at the edge keeps
	/// invalid names (uppercase, spaces, leading/trailing punctuation) a clean
	/// 400 instead of letting them reach the DB CHECK and surface as a 500.
	pub const CONTAINER_REGISTRY_REPOSITORY_NAME_REGEX: &str =
		macros::verify_regex!(r"^[a-z0-9]+((\.|_|__|-+)[a-z0-9]+)*$");

	/// The Regex to validate a container image tag (the `:tag` part of an image
	/// reference). Matches Docker/OCI's tag grammar: 1–128 characters starting
	/// with a word character, followed by word characters, dots, or dashes.
	/// Validating it at the edge keeps invalid or empty tags a clean 400
	/// instead of letting an empty tag reach the DB and produce a broken
	/// `image:` ref.
	pub const DEPLOYMENT_IMAGE_TAG_REGEX: &str =
		macros::verify_regex!(r"^[a-zA-Z0-9_][a-zA-Z0-9._-]{0,127}$");

	/// The Regex to validate a person's first or last name.
	///
	/// Matches 1–100 characters of anything that is not an HTML-relevant
	/// metacharacter (`<`, `>`, `&`), a control character, or a whitespace
	/// other than space (newline, tab, carriage return). Keeps unicode
	/// letters, emoji, apostrophes, hyphens, accents — i.e. the kind of names
	/// real people actually have.
	pub const USER_NAME_REGEX: &str = macros::verify_regex!(r"^[^<>&\n\r\t\x00-\x1f]{1,100}$");

	/// The Regex to validate a role description.
	///
	/// Same metacharacter restrictions as [`USER_NAME_REGEX`], but allows
	/// empty (length 0–500). The role create/update handler substitutes a
	/// default string when the description is empty.
	pub const ROLE_DESCRIPTION_REGEX: &str =
		macros::verify_regex!(r"^[^<>&\n\r\t\x00-\x1f]{0,500}$");
}

#[cfg(test)]
mod tests {
	use std::borrow::Cow;

	use super::validate_password;

	#[test]
	fn valid_password_passes() {
		assert!(validate_password(Cow::Borrowed("SecurePass1@")).is_ok());
	}

	#[test]
	fn missing_digit_fails() {
		let err = validate_password(Cow::Borrowed("SecurePass@")).unwrap_err();
		assert!(
			err.to_string().contains("at least one digit"),
			"expected digit error, got: {}",
			err
		);
	}

	#[test]
	fn missing_lowercase_fails() {
		let err = validate_password(Cow::Borrowed("SECUREPASS1@")).unwrap_err();
		assert!(
			err.to_string().contains("at least one lowercase"),
			"expected lowercase error, got: {}",
			err
		);
	}

	#[test]
	fn missing_uppercase_fails() {
		let err = validate_password(Cow::Borrowed("securepass1@")).unwrap_err();
		assert!(
			err.to_string().contains("at least one uppercase"),
			"expected uppercase error, got: {}",
			err
		);
	}

	#[test]
	fn missing_special_char_fails() {
		let err = validate_password(Cow::Borrowed("SecurePass1")).unwrap_err();
		assert!(
			err.to_string().contains("at least one special character"),
			"expected special char error, got: {}",
			err
		);
	}

	#[test]
	fn all_special_chars_accepted() {
		assert!(validate_password(Cow::Borrowed(r#"aA1@!#$%^&*?"#)).is_ok());
		assert!(validate_password(Cow::Borrowed(r"aA1/\|~`")).is_ok());
		assert!(validate_password(Cow::Borrowed("aA1.,;:<>[]{}")).is_ok());
	}
}
