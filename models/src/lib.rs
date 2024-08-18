//! This crate contains all the DTOs and common models used in the API across
//! the frontend, backend, the CLI, and the controller.

/// All the route definitions
pub mod api;
/// All the CI structs and formats
pub mod ci;
/// Any data that is sent to or from cloudflare (mostly KV)
pub mod cloudflare;
// / All infrastructure as code related structs and formats
// pub mod iaac;
/// All data related to permissions and RBAC data representation
pub mod rbac;
/// Utility functions and structs
pub mod utils;

/// The prelude module contains all the commonly used types and traits that are
/// used across the crate. This is mostly used to avoid having to import a lot
/// of things from different modules.
pub mod prelude {
	pub use headers::UserAgent;
	pub use preprocess;
	pub use tracing::{debug, error, info, instrument, trace, warn};

	pub(crate) use crate as models;
	pub use crate::{
		api::WithId,
		rbac::{
			BillingPermission,
			ContainerRegistryRepositoryPermission,
			DatabasePermission,
			DeploymentPermission,
			DnsRecordPermission,
			DomainPermission,
			ManagedURLPermission,
			Permission,
			RunnerPermission,
			SecretPermission,
			StaticSitePermission,
			VolumePermission,
		},
		utils::{
			AppAuthentication,
			Base64String,
			BearerToken,
			GeoLocation,
			ListOrder,
			LoginId,
			OneOrMore,
			Paginated,
			StringifiedU16,
			TotalCountHeader,
			Uuid,
		},
		ApiEndpoint,
		ApiErrorResponse,
		ApiRequest,
		ApiSuccessResponse,
		ApiSuccessResponseBody,
		AppResponse,
		ErrorType,
	};
}

/// A private module to restrict implementations within the crate.
mod private {
	/// A private trait to restrict implementations within the crate. Any trait
	/// that requires a [`Sealed`] trait to be implemented, must have
	/// implementations provided within the crate. If you need to implement
	/// something outside this crate that requires a [`Sealed`] trait, something
	/// went wrong in the API design.
	#[allow(dead_code)]
	pub trait Sealed {}
}

/// Contains the trait that is used to represent all the data that will be sent
/// to an endpoint in the API. This trait is implemented for all the endpoints
/// in the API. Since it is a large trait, a helper macro is provided to
/// generate a request. See: [`macros::declare_api_endpoint`]
mod endpoint;
/// Contains the enum used to represent an error response from the API. This is
/// an exhaustive list of all the possible error types and the status codes for
/// the error variant
mod error;
/// Contains the struct used to represent a request being made to the API. This
/// struct will contain the path, query, headers, body and it's necessary
/// parameters and fields to make the request. The generated
/// [`request::ApiRequest`] can be used to make requests to the API.
mod request;
/// The structs used to represent all the different types of responses that the
/// API can return, including success responses, error responses, and a
/// flattened enum to parse the response from the API.
mod response;
/// The structs that are used to represent user data in a request. These structs
/// will be used by the audit logger middleware, other middleware, and the API
/// itself.
mod user_data;

pub use self::{endpoint::*, error::*, request::*, response::*, user_data::*};
