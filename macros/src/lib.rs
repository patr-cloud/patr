#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]

//! This crate contains the macros used in this project. It is not intended to
//! be used outside of this project. However, this crate is intended to be a
//! central place for all the macros used in this project, across binaries (CLI,
//! frontend, backend, controller, etc).

use proc_macro::TokenStream;

/// The proc macro for declaring an API endpoint.
mod declare_api_endpoint;
/// The proc macro for declaring an App endpoint for the frontend.
mod declare_app_route;
/// The proc macro for declaring a streaming endpoint. A streaming endpoint is
/// basically a websocket endpoint.
mod declare_stream_endpoint;
/// A derive macro for the `HasHeaders` trait.
mod has_headers;
/// A proc macro for stripping whitespaces and newlines from SQL queries.
mod query;
/// A macro to generate a recursive enum iterator.
mod recursive_enum_iter;
/// An attribute that expands to other attributes for a server fn, adding
/// middlewares to it.
mod server_fn;
/// A macro to verify if a given string is a valid regex at compile time.
mod verify_regex;
/// A macro to get the current crate version.
mod version;

/// Declares an API endpoint.
///
/// This macro allows easy definition of an API endpoint along with the request
/// URL, headers, query, body as well as the response headers and body.
/// Generates the required structs for the endpoint. Currently only supports
/// JSON endpoints.
///
/// ## Example usage:
/// ```rust
/// # use headers::AcceptRanges;
/// # use models::prelude::*;
/// // In the root
/// macros::declare_api_endpoint!(
///     /// The documentation for the endpoint.
///     EndpointName,
///     POST "/:workspace_id/URL/:url_body" {
///         pub workspace_id: Uuid,
///         pub url_body: String,
///     },
///
///     // Can also use paginated_query = ... for automatic pagination
///     query = {
///         pub param1: u32,
///     },
///     request_headers = {
///         pub header1: AcceptRanges,
///         pub token: BearerToken,
///     },
///     request = {
///         pub body_param1: String,
///     },
///
///     // Ref: AuthenticatorType
///     authentication = {
///         AppAuthentication::<Self>::WorkspaceMembershipAuthenticator {
///             extract_workspace_id: |req| req.path.workspace_id,
///         }
///     },
///     response_headers = {
///         pub header1: AcceptRanges,
///     },
///     response = {
///         pub body_param1: String,
///     },
/// );
/// ```
#[proc_macro]
pub fn declare_api_endpoint(input: TokenStream) -> TokenStream {
	declare_api_endpoint::parse(input)
}

/// Declares a app route.
///
/// This macro allows creates a router path, which will be the URL of the page.
/// It includes the URL of the route, a route name, the path parameter e.g
/// /path/:id, the query i.e. the query string, and whether the URL is auth
/// protected or not.
///
/// ## Example usage:
/// ```rust
/// // In the root
/// macros::declare_app_route!(
///     /// The documentation for the endpoint.
///     Login,
///     "/login/:param1" {
///         pub param1: i32
///     },
///     requires_login = true,
///     query = {
///         pub param1: bool,
///     },
/// );
/// ```
#[proc_macro]
pub fn declare_app_route(input: TokenStream) -> TokenStream {
	declare_app_route::parse(input)
}

/// Declares a stream endpoint.
///
/// This macro allows easy definition of a stream endpoint, which is basically a
/// websocket endpoint along with the request URL, headers, query, client
/// message, server message as well as the response headers and body. Generates
/// the required structs for the endpoint.
///
/// ## Example usage:
/// ```rust
/// # use headers::AcceptRanges;
/// # use models::prelude::*;
/// // In the root
/// macros::declare_stream_endpoint!(
///     /// The documentation for the endpoint.
///     EndpointName,
///     GET "/:workspace_id/URL/:url_body" {
///         pub workspace_id: Uuid,
///         pub url_body: String,
///     },
///
///     // Can also use paginated_query = ... for automatic pagination
///     query = {
///         pub param1: u32,
///     },
///     request_headers = {
///         pub header1: AcceptRanges,
///         pub token: BearerToken,
///     },
///     client_msg = {
///         Variant1 {
///             body_param1: String,
///         },
///     },
///
///     // Ref: AuthenticatorType
///     authentication = {
///         AppAuthentication::<Self>::WorkspaceMembershipAuthenticator {
///             extract_workspace_id: |req| req.path.workspace_id,
///         }
///     },
///     response_headers = {
///         pub header1: AcceptRanges,
///     },
///     server_msg = {
///         Variant1 {
///             body_param1: String,
///         },
///     },
/// );
/// ```
#[proc_macro]
pub fn declare_stream_endpoint(input: TokenStream) -> TokenStream {
	declare_stream_endpoint::parse(input)
}

/// A derive macro that makes it easy to implement `HasHeader` for every single
/// field in the given struct.
#[proc_macro_derive(HasHeaders)]
pub fn has_headers(input: TokenStream) -> TokenStream {
	has_headers::parse(input)
}

/// A proc macro that strips whitespaces and newlines from SQL queries. Same as
/// `sqlx::query!` but with the added benefit of stripping whitespaces and
/// newlines.
#[proc_macro]
pub fn query(input: TokenStream) -> TokenStream {
	query::parse(input)
}

/// A macro to generate a recursive enum iterator. This macro generates an
/// iterator for a recursive enum. The enum must be a recursive enum, i.e. it
/// must have a variant that contains the enum itself.
///
/// ## Example usage:
/// ```rust
/// # pub enum AnotherEnum {
/// #    Variant1,
/// #    Variant2,
/// # }
/// #
/// // In the root
/// pub enum RecursiveEnum {
///     Variant1,
///     Variant2(AnotherEnum),
/// }
/// ```
///
/// This will generate an iterator for the given recursive enum.
#[proc_macro_derive(RecursiveEnumIter)]
pub fn recursive_enum_iter(input: TokenStream) -> TokenStream {
	recursive_enum_iter::parse(input)
}

/// A macro to get the current crate version. This is used to set the version
/// number for the current database version
#[proc_macro]
pub fn version(input: TokenStream) -> TokenStream {
	version::parse(input)
}

/// A macro to verify if a given string is a valid regex at compile time.
///
/// ## Example usage:
/// ```rust
/// // In the root
/// macros::verify_regex!(r"^(?:.*[a-z])(?:.*[A-Z])(?:.*\d)(?:.*[@$!%*?&])[A-Za-z\d@$!%*?&]{8,}$");
/// ```
///
/// This will return a compile time error if the given regex is invalid.
#[proc_macro]
pub fn verify_regex(input: TokenStream) -> TokenStream {
	verify_regex::parse(input)
}

/// An attribute that expands to other attributes for a server fn, adding
/// middlewares to it.
/// ## Example usage:
/// ```rust
/// #[server_fn]
/// pub async fn server_fn<E>() -> Result<E::Response, Rejection> where E: ApiEndpoint {
///    Ok(warp::reply::json(&"Hello, World!"))
/// }
/// ```
#[proc_macro_attribute]
pub fn server_fn(args: TokenStream, input: TokenStream) -> TokenStream {
	server_fn::parse(args, input)
}
