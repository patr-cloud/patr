use axum_extra::routing::TypedPath;
use models::utils::Headers;
use preprocess::Preprocessable;
use serde::{Serialize, de::DeserializeOwned};

/// A trait that defines a registry endpoint following the OCI Distribution
/// Specification.
///
/// This trait is similar to `ApiEndpoint` but specialized for registry
/// operations with streaming body support and simplified authentication model.
pub trait RegistryEndpoint
where
	Self: Sized + Clone + Send + 'static,
	Self::RequestPath:
		TypedPath + DeserializeOwned + Preprocessable + Clone + Send + Sync + 'static,
	Self::RequestQuery:
		Serialize + DeserializeOwned + Preprocessable + Clone + Send + Sync + 'static,
	Self::RequestHeaders: Headers + Clone + Send + Sync + 'static,
	Self::ResponseHeaders: Headers + Send + Sync + 'static,
{
	/// The HTTP method for this endpoint
	const METHOD: http::Method;

	/// Whether this endpoint requires authentication
	const REQUIRES_AUTH: bool;

	/// Request path type (e.g., /v2/{name}/manifests/{reference})
	type RequestPath;

	/// Request query type
	type RequestQuery;

	/// Request headers type
	type RequestHeaders;

	/// Response headers type
	type ResponseHeaders;
}
