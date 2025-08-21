/// Handles functions that processes authenticated requests
mod auth_endpoint_handler;
/// Handles the authentication of the requests in case the route is protected
mod authenticator;
/// Handles the creation of a database transaction and a redis connection and
/// passes it to the next layer
mod data_store_connection_handler;
/// Handles functions that processes unauthenticated requests
mod endpoint_handler;
/// Handles the creation of a login id and the validation of the login id. This
/// layer is also responsible for the swapping of the login id in case it is
/// required, as described in the documentation for
/// [`UserWebLogin`][models::api::user::UserWebLogin]
mod login_id_manager;
/// Handles the preprocessing of the request, such as the validation of the
/// request body and returning the error if the request body is invalid
mod preprocess_handler;
/// Handles the parsing of the request in the required format and passes a
/// [`ApiRequest`][ApiRequest] to the next layer
mod request_parser;
/// The layer that validates the user agent of the request and makes sure that
/// the user agent is a browser and not a bot in case the user is accessing from
/// the web dashboard. This is also used to make sure that requests that cannot
/// be accessed by the API are only accessed by the web dashboard
mod user_agent_validation_layer;

pub use self::{
	auth_endpoint_handler::*,
	authenticator::*,
	data_store_connection_handler::*,
	endpoint_handler::*,
	login_id_manager::*,
	preprocess_handler::*,
	request_parser::*,
	user_agent_validation_layer::*,
};
