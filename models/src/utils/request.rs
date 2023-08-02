use crate::api::ApiEndpoint;

pub struct ApiRequest<E>
where
	E: ApiEndpoint,
{
	pub path: E::RequestPath,
	pub query: E::RequestQuery,
	// pub headers: E::RequestHeaders,
	pub body: E::RequestBody,
}
