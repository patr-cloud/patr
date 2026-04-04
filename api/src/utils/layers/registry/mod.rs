/// Handles the authentication of the requests in case the route is protected
mod authenticator;
/// Handles the creation of a database transaction and a redis connection and
/// passes it to the next layer
mod data_store_connection_handler;
/// Handles functions that processes unauthenticated requests for the registry
mod endpoint_handler;
/// Handles the preprocessing of the request, such as the validation of the
/// request body and returning the error if the request body is invalid
mod preprocess_layer;
/// Rate limiter layer for registry endpoints using Redis sorted sets
mod rate_limiter_layer;
/// Handles the parsing of the request in the required format and passes a
/// [`RegistryRequest`] to the next layer
mod request_parser;

pub use self::{
	authenticator::*,
	data_store_connection_handler::*,
	endpoint_handler::*,
	preprocess_layer::*,
	rate_limiter_layer::*,
	request_parser::*,
};
