use crate::ApiEndpoint;

pub struct ApiRequest<E>
where
	E: ApiEndpoint,
{
	pub path: E::RequestPath,
	pub query: E::RequestQuery,
	pub headers: E::RequestHeaders,
	pub body: E::RequestBody,
}

impl<T> ApiRequest<T>
where
	T: ApiEndpoint,
	T::RequestPath: Default,
{
	pub fn new(
		query: T::RequestQuery,
		headers: T::RequestHeaders,
		body: T::RequestBody,
	) -> Self {
		Self {
			path: Default::default(),
			query,
			headers,
			body,
		}
	}
}

impl<T> ApiRequest<T>
where
	T: ApiEndpoint,
{
	pub fn new_with_path(
		path: T::RequestPath,
		query: T::RequestQuery,
		headers: T::RequestHeaders,
		body: T::RequestBody,
	) -> Self {
		Self {
			path,
			query,
			headers,
			body,
		}
	}
}
