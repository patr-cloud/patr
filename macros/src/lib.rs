#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! This crate contains the macros used in this project. It is not intended to
//! be used outside of this project. However, this crate is intended to be a
//! central place for all the macros used in this project, across binaries (CLI,
//! frontend, backend, controller, etc).

use proc_macro::TokenStream;

/// The proc macro for declaring an API endpoint.
mod declare_api_endpoint;
/// A derive macro for the `HasHeaders` trait.
mod has_headers;

/// Declares an API endpoint. This macro allows easy definition of an API
/// endpoint along with the request URL, headers, query, body as well as the
/// response headers and body. Generates the required structs for the endpoint.
/// Currently only supports JSON endpoints.
///
/// ## Example usage:
/// ```rust
/// // In the root
/// macros::declare_api_endpoint!(
///     /// The documentation for the endpoint.
///     EndpointName,
///     Method "/URL" {
///         pub url_body: IfAny,
///     },
///
///     // Can also use paginated_query = ... for automatic pagination
///     query = {
///         pub param1: Type,
///     },
///     request_headers = {
///         pub header1: Type,
///     },
///     request = {
///         pub body_param1: Type,
///     },
///
///     // Ref: AuthenticatorType
///     authentication = WorkspaceMembershipAuthenticator {
///         extract_workspace_id: |req| req.path.workspace_id,
///     },
///     response_headers = {
///         pub header1: Type,
///     },
///     response = {
///         pub body_param1: Type,
///     },
/// );
/// ```
#[proc_macro]
pub fn declare_api_endpoint(input: TokenStream) -> TokenStream {
	declare_api_endpoint::parse(input)
}

/// A derive macro that makes it easy to implement `HasHeader` for every single
/// field in the given struct.
#[proc_macro_derive(HasHeaders)]
pub fn has_headers(input: TokenStream) -> TokenStream {
	has_headers::parse(input)
}
