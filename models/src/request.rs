use std::marker::PhantomData;

use http::Method;
use leptos::server_fn::codec::{Encoding, GetUrl};
use preprocess::Preprocessable;
use typed_builder::TypedBuilder;

use crate::prelude::*;

/// This is the API encoding used to encode and decode the body of the request.
#[derive(Debug, Clone, Copy, Default)]
pub struct ApiEncoding<E>(PhantomData<E>)
where
	E: ApiEndpoint;

impl<E> Encoding for ApiEncoding<E>
where
	E: ApiEndpoint,
{
	const CONTENT_TYPE: &'static str = if E::IS_REST {
		GetUrl::CONTENT_TYPE
	} else {
		// If the response body is not a REST API, then we can't know the content type
		// of the response. So we just return the default content type of binary data.
		"application/octet-stream"
	};
	const METHOD: Method = E::FRONTEND_API_METHOD;
}

/// This struct represents a request to the API. It contains the path, query,
/// headers and body of the request. This struct provides a builder API to make
/// it easier to construct requests.
#[derive(TypedBuilder)]
pub struct ApiRequest<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The path of the request. This is the part of the URL after the domain
	/// and port.
	pub path: E::RequestPath,
	/// The query of the request. This is the part of the URL after the `?`.
	pub query: E::RequestQuery,
	/// The headers of the request.
	pub headers: E::RequestHeaders,
	/// The body of the request. This is the actual data that was sent by the
	/// client. Can be either JSON or Websockets.
	pub body: E::RequestBody,
}
