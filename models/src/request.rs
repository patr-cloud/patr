use std::default::Default;

use preprocess::Preprocessable;
use typed_builder::TypedBuilder;

use crate::prelude::*;

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
	#[builder(default, default_where(E::RequestPath: Default))]
	pub path: E::RequestPath,
	/// The query of the request. This is the part of the URL after the `?`.
	#[builder(default, default_where(E::RequestQuery: Default))]
	pub query: E::RequestQuery,
	/// The headers of the request.
	#[builder(default, default_where(E::RequestHeaders: Default))]
	pub headers: E::RequestHeaders,
	/// The body of the request. This is the actual data that was sent by the
	/// client. Can be either JSON or Websockets.
	#[builder(default, default_where(E::RequestBody: Default))]
	pub body: E::RequestBody,
}

/// This struct represents a preprocessed request to the API. It contains the
/// path, query, headers and preprocessed body of the request. This struct
/// provides a builder API to make it easier to construct requests.
#[derive(TypedBuilder)]
pub struct ProcessedApiRequest<E>
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
	pub body: <E::RequestBody as Preprocessable>::Processed,
}

impl<E> TryFrom<ApiRequest<E>> for ProcessedApiRequest<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	type Error = preprocess::Error;

	fn try_from(value: ApiRequest<E>) -> Result<Self, Self::Error> {
		let ApiRequest {
			path,
			query,
			headers,
			body,
		} = value;
		Ok(ProcessedApiRequest {
			path,
			query,
			headers,
			body: body.preprocess()?,
		})
	}
}
