use std::fmt::Debug;

use axum_extra::routing::TypedPath;
use serde::{de::DeserializeOwned, Serialize};

use crate::utils::{
	HasHeaders,
	Headers,
	IntoAxumResponse,
	RequiresRequestHeaders as RequestHeaders,
	RequiresResponseHeaders as ResponseHeaders,
};

pub trait ApiEndpoint
where
	Self: Sized + Clone + Send + 'static,
	Self::RequestPath: TypedPath
		+ ResponseHeaders
		+ Serialize
		+ DeserializeOwned
		+ Clone
		+ Send
		+ Sync
		+ 'static,
	Self::RequestQuery: ResponseHeaders
		+ Serialize
		+ DeserializeOwned
		+ Clone
		+ Send
		+ Sync
		+ 'static,
	Self::RequestHeaders: Headers
		+ ResponseHeaders
		+ HasHeaders<
			<Self::ResponseBody as RequestHeaders>::RequiredRequestHeaders,
		> + Clone
		+ Send
		+ Sync
		+ 'static,
	Self::RequestBody: ResponseHeaders
		+ Serialize
		+ DeserializeOwned
		+ Clone
		+ Send
		+ Sync
		+ 'static,

	Self::ResponseHeaders: Headers
		+ HasHeaders<
			<Self::RequestPath as ResponseHeaders>::RequiredResponseHeaders,
		> + HasHeaders<
			<Self::RequestQuery as ResponseHeaders>::RequiredResponseHeaders,
		> + HasHeaders<
			<Self::RequestBody as ResponseHeaders>::RequiredResponseHeaders,
		> + HasHeaders<
			<Self::RequestHeaders as ResponseHeaders>::RequiredResponseHeaders,
		> + Debug
		+ Send
		+ Sync
		+ 'static,
	Self::ResponseBody: IntoAxumResponse
		+ RequestHeaders
		+ ResponseHeaders
		+ Debug
		+ Send
		+ Sync
		+ 'static,
{
	const METHOD: reqwest::Method;
	type RequestPath;
	type RequestQuery;
	type RequestHeaders;
	type RequestBody;

	type ResponseHeaders;
	type ResponseBody;
}
