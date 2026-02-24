/// The layer that handles the audit logging of the requests. This layer is
/// responsible for logging the actions performed on the endpoint, including the
/// request, the response, and the user that performed the action. This layer is
/// only used for endpoints that require auditing, as specified by the
/// [`AppAuditLogger`][1] struct in the [`ApiEndpoint`][2] trait.
///
/// [1]: models::utils::AppAuditLogger
/// [2]: models::prelude::ApiEndpoint
mod audit_logger_layer;
/// Handles functions that processes authenticated requests
mod auth_endpoint_handler;
/// Handles the authentication of the requests in case the route is protected
mod authenticator;
/// Handles the authorization of the requests in case the route requires
/// specific permissions
mod authorizer;
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
mod preprocess_layer;
/// Handles the parsing of the request in the required format and passes a
/// [`ApiRequest`][ApiRequest] to the next layer
mod request_parser;
/// The layer that validates the user agent of the request and makes sure that
/// the user agent is a browser and not a bot in case the user is accessing from
/// the web dashboard. This is also used to make sure that requests that cannot
/// be accessed by the API are only accessed by the web dashboard
mod user_agent_validation_layer;
/// The layer that manages the auth cookie for web dashboard requests. This
/// layer is responsible for setting the auth cookie and automatically
/// refreshing it, along with setting it as the authentication header for
/// downstream layers.
mod web_dashboard_auth_cookie_layer;

/// All layers that are used by the Patr Registry.
pub mod registry;

pub use self::{
	audit_logger_layer::*,
	auth_endpoint_handler::*,
	authenticator::*,
	authorizer::*,
	data_store_connection_handler::*,
	endpoint_handler::*,
	login_id_manager::*,
	preprocess_layer::*,
	request_parser::*,
	user_agent_validation_layer::*,
	web_dashboard_auth_cookie_layer::*,
};
