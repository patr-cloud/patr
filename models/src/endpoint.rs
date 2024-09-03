use std::fmt::Debug;

use axum_extra::routing::TypedPath;
use preprocess::Preprocessable;
use serde::{de::DeserializeOwned, Serialize};

use crate::utils::{
	FromAxumRequest,
	HasHeaders,
	Headers,
	IntoAxumResponse,
	RequiresRequestHeaders as RequestHeaders,
	RequiresResponseHeaders as ResponseHeaders,
};

/// A trait that defines an API endpoint.
///
/// This is used to generate the routes for the API, as well as the
/// corresponding path, request query, request headers, request body, response
/// headers, and response body types.
///
/// Ideally, this trait would contain all the information needed to define the
/// functionality of a route
pub trait ApiEndpoint
where
	Self: Sized + Clone + Send + 'static,
	Self::RequestPath:
		TypedPath + ResponseHeaders + Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
	Self::RequestQuery:
		ResponseHeaders + Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
	Self::RequestHeaders: Headers
		+ ResponseHeaders
		+ HasHeaders<<Self::ResponseBody as RequestHeaders>::RequiredRequestHeaders>
		+ HasHeaders<<Self::Authenticator as RequestHeaders>::RequiredRequestHeaders>
		+ Clone
		+ Send
		+ Sync
		+ 'static,
	Self::RequestBody: ResponseHeaders + FromAxumRequest + Preprocessable + Send + Sync + 'static,
	<Self::RequestBody as Preprocessable>::Processed: Send,
	Self::Authenticator: RequestHeaders + Clone + Send,

	Self::ResponseHeaders: Headers
		+ HasHeaders<<Self::RequestPath as ResponseHeaders>::RequiredResponseHeaders>
		+ HasHeaders<<Self::RequestQuery as ResponseHeaders>::RequiredResponseHeaders>
		+ HasHeaders<<Self::RequestBody as ResponseHeaders>::RequiredResponseHeaders>
		+ HasHeaders<<Self::RequestHeaders as ResponseHeaders>::RequiredResponseHeaders>
		+ Debug
		+ Send
		+ Sync
		+ 'static,
	Self::ResponseBody:
		IntoAxumResponse + RequestHeaders + ResponseHeaders + Debug + Send + 'static,
{
	/// The HTTP method that should be used for this endpoint
	const METHOD: http::Method;
	/// If true, this route can be accessed by the API. Otherwise, it'll only be
	/// accessible by the Web UI
	const API_ALLOWED: bool;

	/// The path that should be used for this endpoint. This should be a valid
	/// HTML URL Path and can contain URL parameters as a struct. For example,
	/// `/users/:id` would be a valid path. However, the provided struct must
	/// implement [`serde::Deserialize`] and [`serde::Serialize`], in order to
	/// parse and serialize the URL parameters. This is internally implemented
	/// using [`axum_extra::routing::TypedPath`]
	type RequestPath;
	/// The query that should be used for this endpoint. This should be a valid
	/// HTML URL Query and can contain any query parameters that can be
	/// serialized and deserialized by [`serde_urlencoded`]
	type RequestQuery;
	/// The request headers that should be used for this endpoint. This should
	/// be a struct that implements [`Headers`]. For ease of use, a derive macro
	/// is provided for this trait ([`macros::HasHeaders`]). Each field in this
	/// struct should be a valid header and should implement
	/// [`typed_headers::Header`]
	type RequestHeaders;
	/// The request body that should be used for this endpoint. This should be a
	/// struct that implements [`serde::Deserialize`] and [`serde::Serialize`].
	/// Any request should be of JSON type.
	///
	/// TODO: Later on, allow stream requests, such as `multipart/form-data`
	type RequestBody;

	/// The authenticator that should be used for this endpoint. This should be
	/// a struct that implements the [`HasAuthentication`] trait. This is a
	/// sealed trait, meaning it cannot be implemented outside of this crate.
	/// There are two structs provided that implement this trait and no other
	/// structs can implement this trait. These structs are:
	/// - [`NoAuthentication`][1]: This struct is used to specify that an API
	///   endpoint does not require authentication. It can be accessed without
	///   any token.
	/// - [`AppAuthentication`][2]: This struct is used to specify that an API
	///   endpoint requires authentication. It can be accessed only with a valid
	///   token. This token can be of three types:
	///     - [`PlainTokenAuthenticator`][3]: Any logged in user can access this
	///       endpoint.
	///     - [`WorkspaceMembershipAuthenticator`][4]: Only users that are
	///       members of the workspace that is specified in the
	///       [`extract_workspace_id`][5] function can access this endpoint.
	///     - [`ResourcePermissionAuthenticator`][6]: Only users that have the
	///       specified permission on the resource that is specified in the
	///       request can access this endpoint.
	///
	/// [1]: crate::utils::NoAuthentication
	/// [2]: crate::utils::AppAuthentication
	/// [3]: crate::utils::AppAuthentication::PlainTokenAuthenticator
	/// [4]: crate::utils::AppAuthentication::WorkspaceMembershipAuthenticator
	/// [5]: crate::utils::AppAuthentication::WorkspaceMembershipAuthenticator::extract_workspace_id
	/// [6]: crate::utils::AppAuthentication::ResourcePermissionAuthenticator
	type Authenticator;

	/// The response headers that should be used for this endpoint. This should
	/// be a struct that implements [`Headers`]. For ease of use, a derive macro
	/// is provided for this trait ([`macros::HasHeaders`]). Each field in this
	/// struct should be a valid header and should implement
	/// [`typed_headers::Header`]
	type ResponseHeaders;
	/// The response body that should be used for this endpoint. This should be
	/// a struct that implements [`IntoAxumResponse`]. This can either be a JSON
	/// response or a stream response, such as websockets.
	type ResponseBody;

	/// The authenticator that should be used for this endpoint. This should be
	/// a struct that implements the [`HasAuthentication`] trait
	fn get_authenticator() -> Self::Authenticator
	where
		Self::Authenticator: Default,
	{
		Self::Authenticator::default()
	}
}
