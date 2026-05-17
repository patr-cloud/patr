#![forbid(unsafe_code)]

//! This crate contains the macros used in this project. It is not intended to
//! be used outside of this project. However, this crate is intended to be a
//! central place for all the macros used in this project, across binaries (CLI,
//! frontend, backend, controller, etc).

use proc_macro::TokenStream;

/// The proc macro for declaring an API endpoint.
mod declare_api_endpoint;
/// The proc macro for declaring a registry endpoint.
mod declare_registry_endpoint;
/// The proc macro for declaring a streaming endpoint. A streaming endpoint is
/// basically a websocket endpoint.
mod declare_stream_endpoint;
/// A macro to generate email templates. This is used to generate the email
/// templates to send in the background worker.
mod email_template;
/// A derive macro for the `HasHeaders` trait.
mod has_headers;
/// A derive macro to generate an enum of all fields and a search struct
/// for a given struct.
mod listable_resource;
/// An attribute macro to register a database migration. Derives the migration
/// name from the source filename and version from the parent directory.
mod migration;
/// A macro to generate the same struct but with all fields optional.
mod optionalize;
/// A proc macro for stripping whitespaces and newlines from SQL queries.
mod query;
/// A macro to generate a recursive enum iterator.
mod recursive_enum_iter;
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
///     POST "/{workspace_id}/URL/{url_body}" {
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

/// Declares a registry endpoint.
///
/// This macro allows easy definition of a registry endpoint along with the
/// request URL, headers, query, body as well as the response headers and body.
/// Generates the required structs for the endpoint. Generates the OCI compliant
/// structs for the endpoint.
///
/// ## Example usage:
/// ```rust
/// # use headers::AcceptRanges;
/// # use models::prelude::*;
/// // In the root
/// macros::declare_registry_endpoint!(
///     /// The documentation for the endpoint.
///     GetManifest,
///     GET "/v2/{workspace_id}/{name}/manifests/{reference}" {
///         pub workspace_id: Uuid,
///         pub name: String,
///         pub reference: String,
///     },
///     request_headers = {
///         pub token: BearerToken,
///     },
///     request = {
///         pub body_param1: String,
///     },
/// );
/// ```
#[proc_macro]
pub fn declare_registry_endpoint(input: TokenStream) -> TokenStream {
	declare_registry_endpoint::parse(input)
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
///     GET "/{workspace_id}/URL/{url_body}" {
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

/// A derive macro that makes it easy to implement `ListableResource` for a
/// struct. This macro generates an enum of all fields and a search struct for
/// a given struct. The generated enum can be used to filter the results of a
/// paginated request. The generated search struct can be used to filter the
/// results of a paginated request as well.
///
/// ## Example usage:
/// ```rust
/// # use models::prelude::*;
/// #[derive(ListableResource)]
/// pub struct User {
///     pub name: String,
///     pub age: u32,
/// }
/// ```
///
/// This will generate an enum `UserFieldList` with the variants `Name` and
/// `Age`, and a struct `UserSearch` with the fields `name` and `age`. The
/// `UserFieldList` enum can be used to filter the results of a paginated
/// request and the `UserSearch` struct can be used to filter the results of a
/// paginated request as well. The generated enum and struct will also implement
/// the `ListableResource` trait, which can be used to define the fields that
/// can be used to sort the resource in a paginated request.
#[proc_macro_derive(ListableResource, attributes(sortable, search))]
pub fn listable_resource(input: TokenStream) -> TokenStream {
	listable_resource::parse(input)
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

/// Registers a database migration. Place on an async function that takes
/// `(&mut DatabaseConnection, &AppConfig)` and returns `Result<(), ErrorType>`.
///
/// The migration name is derived from the source filename and the version
/// from the parent directory name (`v{major}_{minor}_{patch}/`).
///
/// ## Example usage:
/// ```rust
/// #[macros::migration]
/// async fn migrate(
///     connection: &mut DatabaseConnection,
///     _config: &AppConfig,
/// ) -> Result<(), ErrorType> {
///     // migration SQL here
///     Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn migration(args: TokenStream, input: TokenStream) -> TokenStream {
	migration::parse(args, input)
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

/// A macro to generate the same struct but with all fields optional.
/// This is useful for creating a struct that can be used to update an existing
/// struct, where all fields are optional.
/// ## Example usage:
/// ```rust
/// # use ::macros::optionalize;
/// #[optionalize]
/// pub struct User {
///     pub name: String,
///     pub age: u32,
/// }
/// ```
/// This will generate a struct `UserOptional` with all fields optional.
/// The generated struct will have the same fields as the original struct, but
/// all fields will be wrapped in `Option`. The generated struct will also
/// have a few utility methods, such as `any_field_set` to check if any field is
/// set, `all_fields_set` to check if all fields are set, and an implementation
/// of `models::utils::Optionalizable` for the original struct.
/// You can skip individual fields in the generated struct by annotating the
/// original field with `#[optionalize(skip)]`.
/// You can keep an already optional field unchanged with
/// `#[optionalize(keep)]`.
/// Place `#[optionalize]` before other active struct attributes like
/// `#[derive(...)]` to apply them to both original and generated structs.
#[proc_macro_attribute]
pub fn optionalize(args: TokenStream, input: TokenStream) -> TokenStream {
	optionalize::parse(args, input)
}

/// A derive macro that generates an `.into_email_body()` method for email
/// template structs.
///
/// The macro generates two internal Askama wrapper structs (one for `.mjml`,
/// one for `.txt`) with the same fields, plus a subject template. The
/// `.into_email_body()` method renders all three and returns an `EmailBody`
/// containing the subject, HTML (via MJML), and plain text.
///
/// # Example usage:
/// ```rust
/// use macros::EmailTemplate;
///
/// #[derive(EmailTemplate)]
/// #[template(path = "user-sign-up", subject = "Verify your email, {{ username }} | Patr")]
/// pub struct UserSignUpEmail {
///     pub username: String,
///     pub otp: String,
///     pub otp_expiry: String,
/// }
/// ```
#[proc_macro_derive(EmailTemplate, attributes(template))]
pub fn email_template(input: TokenStream) -> TokenStream {
	email_template::parse(input)
}
