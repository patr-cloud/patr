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

	/// The regular expression used to validate a username. The username must
	/// start with an alphanumeric character or an underscore, and end with an
	/// alphanumeric character. The username can contain alphanumeric
	/// characters, underscores, dots, and hyphens.
	pub const USERNAME_VALIDITY_REGEX: &str = r"^[a-z0-9_][a-z0-9_\.\-]*[a-z0-9_]$";

	/// Regex to validate The Country Code of the phone number. The country code
	/// must start with a plus sign followed by 1 to 4 digits.
	pub const PHONE_NUMBER_COUNTRY_CODE_REGEX: &str = r"^\+\d{1,4}$";

	/// The Regex to validate the phone number. The phone number must be in the
	/// standard 10-digit number format. The number must be in the format `(123)
	/// 456 7890`, `123-456-7890, 1234567890, 123.456.7890`,
	pub const PHONE_NUMBER_REGEX: &str = r"^\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}$";

	/// The Regex to validate the password. The password must have a minimum of
	/// 6 characters Must contain atleast one digit, one uppercase letter, one
	/// lowercase letter and one special character (!@#$%^&*?)
	///
	/// Explanation:
	/// ```
	/// ^			// Start of the line
	/// (?=\S{8,})		// Atleast 8 characters
	/// (?=\S*\d)		// Atleast one digit
	/// (?=\S*[A-Z])		// Atleast one uppercase letter
	/// (?=\S*[a-z])		// Atleast one lowercase letter
	/// (?=\S*[!@//$%^&*?])	// Atleast one special character
	/// \S*			// 0 or more non-space characters with previous conditions in mind
	/// $			// End of the line
	/// ```
	pub const PASSWORD_REGEX: &str =
		r"^\S*(?=\S{8,})(?=\S*\d)(?=\S*[A-Z])(?=\S*[a-z])(?=\S*[!@#$%^&*?])\S*$";

	/// The Regex to validate OTP of the user. The OTP must be a 6-digit number.
	/// The OTP can be of the format `123456` or `123-456`.
	pub const OTP_VERIFICATION_TOKEN_REGEX: &str = r"^(\d{3}\-?\d{3})$";

	/// The Regex to validate a resource name (e.g. deployment name, etc.)
	/// Matches a string that is between 4 and 255 characters long and can have
	/// digits, small letters, hyphens, and underscores.

	// PREVIOUSLY: ^[a-zA-Z0-9_\\-\\.][a-zA-Z0-9_\\-\\. ]{0,62}[a-zA-Z0-9_\\-\\.]$
	pub const RESOURCE_NAME_REGEX: &str = r"^[a-z0-9\-_]{4,255}$";

	pub const DNS_RECORD_NAME_REGEX: &str = r"^((@)|(\\*)|((\\*\\.)?(([a-z0-9_]|[a-z0-9_][a-z0-9_\\-]*[a-z0-9_])\\.)*([a-z0-9_]|[a-z0-9_][a-z0-9_\\-]*[a-z0-9_])))$";

	/// The Regex to validate file names (e.g. for satic sites)
	///
	/// Explanation:
	/// ```
	/// ^			// Start of the line
	/// (
	/// 	\/?		// Optional forward slash
	/// 	\w		// A word character (This is to ensure that there are no spaces in the file name)
	/// 	[\w\s]*		// One or more word characters or spaces
	/// )*			// Zero or more of the previous group (This allows for zero or more directories)
	/// \.			// A period
	/// [\w]{2,}		// Two or more word characters (This is to ensure that the file has an extension)
	/// ```

	pub const FILE_NAME_REGEX: &str = r"^(\/?\w[\w\s]*)*\.[\w]{2,}";
}

pub fn validate_token(value: String) -> Result<String, ::preprocess::Error> {
	if value.len() != 6 && value.parse::<u32>().is_ok() {
		return Err(::preprocess::Error::new("Invalid verification token"));
	}
	Ok(value)
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
