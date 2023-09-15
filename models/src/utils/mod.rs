mod axum_request;
mod axum_response;
mod bools;
mod header_utils;
mod middlewares;
mod one_or_many;
mod paginated;
mod tuple_utils;
mod uuid;
mod websocket;

pub use self::{
	axum_request::*,
	axum_response::*,
	bools::*,
	header_utils::*,
	middlewares::*,
	one_or_many::*,
	paginated::*,
	tuple_utils::*,
	uuid::*,
	websocket::*,
};

/// All the constants used in the application.
/// Constants are used to avoid hardcoding values, since that might introduce
/// typos.
pub mod constants {
	/// Base URL for the API
	pub const API_BASE_URL: &str = "https://api.patr.cloud";
}
