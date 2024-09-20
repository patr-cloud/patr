/// Authenticates the request and forwards it to the next layer, if successful.
mod authenticator;
/// Starts a database transaction and forwards it to the next layer.
///
/// If the request returns an error, the transaction will be automatically
/// rolled back. If the request is successful, the transaction will be
/// committed.
mod data_store_connection_handler;
/// Handles the endpoint of the request
mod endpoint_handler;
/// Preprocesses the request and forwards it to the next layer
mod preprocess_handler;
/// Removes an IP address from the request and forwards it to the next layer
mod remove_ip_addr;

pub use self::{
	authenticator::*,
	data_store_connection_handler::*,
	endpoint_handler::*,
	preprocess_handler::*,
	remove_ip_addr::*,
};
