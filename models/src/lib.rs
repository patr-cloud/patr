#![feature(adt_const_params)]

//! This crate contains all the DTOs and common models used in the API across
//! the frontend, backend, the CLI, and the controller.

/// All the route definitions
pub mod api;
/// All the CI structs and formats
pub mod ci;
/// Any data that is sent to or from cloudflare (mostly KV)
pub mod cloudflare;
/// All infrastructure as code related structs and formats
pub mod iaac;
/// All data related to permissions and RBAC data representation
pub mod rbac;
/// Utility functions and structs
pub mod utils;

/// The prelude module contains all the commonly used types and traits that are
/// used across the crate. This is mostly used to avoid having to import a lot
/// of things from different modules.
pub mod prelude {
	pub use headers::UserAgent;
	pub(crate) use macros::ListableResource;
	pub use preprocess;
	pub use tracing::{debug, error, info, instrument, trace, warn};

	pub(crate) use crate as models;
	pub use crate::{
		ApiErrorResponse,
		ApiRequest,
		ApiSuccessResponseBody,
		AppResponse,
		ErrorType,
		ProcessedApiRequest,
		api::{ApiEndpoint, OnlyId, WithId},
		rbac::{
			BillingPermission,
			ContainerRegistryRepositoryPermission,
			DatabasePermission,
			DeploymentPermission,
			DnsRecordPermission,
			DomainPermission,
			ManagedURLPermission,
			Permission,
			ResourceType,
			RunnerPermission,
			SecretPermission,
			StaticSitePermission,
			VolumePermission,
		},
		utils::{
			AppAuthentication,
			AuditLogType,
			AuditLogger,
			Base64String,
			BearerToken,
			DeduplicatedIaacResourceExt,
			GeoLocation,
			IsEmpty,
			ListResourceQuery,
			Location,
			LoginId,
			OneOrMore,
			OrderedIaacResourceExt,
			Range,
			ResourceIdExtractor,
			SortOrder,
			StringifiedU16,
			TotalCountHeader,
			TryIteratorExt,
			Uuid,
		},
	};
}

/// A private module to restrict implementations within the crate.
mod private {
	/// A private trait to restrict implementations within the crate. Any trait
	/// that requires a [`Sealed`] trait to be implemented, must have
	/// implementations provided within the crate. If you need to implement
	/// something outside this crate that requires a [`Sealed`] trait, something
	/// went wrong in the API design.
	#[expect(dead_code)]
	pub trait Sealed {}
}

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

pub use self::{error::*, request::*, response::*, user_data::*};
