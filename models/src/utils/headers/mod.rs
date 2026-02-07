use headers::Header;
use http::HeaderMap;
use ts_rs::TS;

/// The header that represents a Bearer token for authentication.
mod bearer_token;
/// The header that represents the Docker-Content-Digest header.
/// This is used in manifest responses to indicate the digest of the
/// manifest.
mod docker_content_digest;
/// The header that represents the Docker-Distribution-API-Version header.
/// This is used in the version check endpoint to indicate the version of the
/// Docker Distribution API that the registry supports.
mod docker_distribution_api_version;
/// The header that represents the Docker upload UUID for blob uploads.
mod docker_upload_uuid;
/// A submodule for implementing the [`HasHeaders`] trait.
mod has_headers;
/// The header that represents the `Location` header.
mod location;
/// The header that represents a Login ID.
mod login_id;
/// A submodule for implementing an optional header that may or may not be
/// present in the request/response.
mod optional_header;
/// The header that represents an HTTP `Range` header.
mod range;
/// A submodule for implementing the [`RequiresRequestHeaders`] and
/// [`RequiresResponseHeaders`] traits.
mod requires_headers;
/// The header that represents the total count of items in a paginated response.
mod total_count_header;

pub use self::{
	bearer_token::*,
	docker_content_digest::*,
	docker_distribution_api_version::*,
	docker_upload_uuid::*,
	has_headers::*,
	location::*,
	login_id::*,
	optional_header::*,
	range::*,
	requires_headers::*,
	total_count_header::*,
};

/// This trait is used to convert a struct to and from a [`HeaderMap`].
///
/// This is mostly used for internal purposes and you shouldn't have to
/// implement this by hand. This is automatically implemented for any endpoint
/// defined using the [`macros::declare_api_endpoint`] macro.
pub trait Headers: Sized {
	/// Convert the struct to a [`HeaderMap`].
	fn to_header_map(&self) -> HeaderMap;
	/// Convert the struct from a [`HeaderMap`], returning a [`None`] if the
	/// conversion fails.
	///
	/// # Errors
	/// Returns an error if the conversion fails, or if the header map is
	/// invalid.
	fn from_header_map(map: HeaderMap) -> Result<Self, headers::Error>;
}

impl Headers for () {
	fn to_header_map(&self) -> HeaderMap {
		HeaderMap::new()
	}

	fn from_header_map(_: HeaderMap) -> Result<Self, headers::Error> {
		Ok(())
	}
}

/// A wrapper struct that is used to export the headers of a struct using
/// [`ts_rs`].
///
/// This is used in the [`macros::declare_api_endpoint`] macro to export the
/// request and response headers of an endpoint with the header name as the
/// field key and the type as string.
pub struct HeaderExporter<T>(pub T)
where
	T: Header;

impl<T> TS for HeaderExporter<T>
where
	T: Header,
{
	type OptionInnerType = Self;
	type WithoutGenerics = String;

	fn decl() -> String {
		"string".to_string()
	}

	fn decl_concrete() -> String {
		"string".to_string()
	}

	fn name() -> String {
		"string".to_string()
	}

	fn inline() -> String {
		panic!("HeaderExporter cannot be inlined")
	}

	fn inline_flattened() -> String {
		format!("\"{}\": string,", <T as Header>::name().as_str())
	}
}
