/// A module containing the client that is used to make requests to the backend
mod client;

/// A module containing the custom [`AppRoute`].
/// The App Route Enum. This Enum is used to specify the route of the app.
mod app_route;
/// A module containing extension traits for various types
mod ext_traits;
mod hooks;
mod routes;
mod sidebar_items;
mod storage;

pub use self::{app_route::*, ext_traits::*, hooks::*, routes::*, sidebar_items::*, storage::*};

/// A module containing constants that are used throughout the application.
pub mod constants {
	use semver::Version;

	/// The version of the application
	pub const VERSION: Version = macros::version!();
	/// The name of the cookie that stores the auth state
	pub const AUTH_STATE: &str = "authState";
}
