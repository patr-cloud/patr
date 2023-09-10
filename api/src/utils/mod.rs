/// The configuration data that is loaded when the backend starts. This contains
/// the details of the database, Redis, the JWT secret, etc.
pub mod config;
/// Contains the [`layer`][1]s that will be used with [`tower`] mounted on the
/// axum [`Router`][2]
///
/// [1]: tower::Layer
/// [2]: axum::Router
pub mod layers;

mod router_ext;

pub use self::router_ext::RouterExt;

/// A list of constants that will be used throughout the application. This is
/// mostly kept to prevent typos.
pub mod constants {
	/// The issuer (iss) of the JWT. This is currently the URL of Patr API.
	pub const JWT_ISSUER: &str = "https://api.patr.cloud";
}
