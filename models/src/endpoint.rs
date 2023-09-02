use std::fmt::Debug;

use axum_extra::routing::TypedPath;
use serde::{de::DeserializeOwned, Serialize};

use crate::utils::{
	AuthenticationType,
	HasHeaders,
	Headers,
	IntoAxumResponse,
	RequiresRequestHeaders as RequestHeaders,
	RequiresResponseHeaders as ResponseHeaders,
};

/// A trait that defines an API endpoint. This is used to generate the routes
/// for the API, as well as the corresponding path, request query, request
/// headers, request body, response headers, and response body types.
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
		+ Clone
		+ Send
		+ Sync
		+ 'static,
	Self::RequestBody:
		ResponseHeaders + Serialize + DeserializeOwned + Clone + Send + Sync + 'static,

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
		IntoAxumResponse + RequestHeaders + ResponseHeaders + Debug + Send + Sync + 'static,
{
	/// The HTTP method that should be used for this endpoint
	const METHOD: reqwest::Method;
	const AUTHENTICATION: AuthenticationType<Self>;
	const AUTHD: bool = false;

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
}
