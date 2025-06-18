#[cfg(not(target_arch = "wasm32"))]
/// The client module. This module is used to communicate with the server
/// and fetch data from the server. It is used to make API calls for the
/// backend.
mod client;
/// The color enum. This enum is used to specify the color of a component. These
/// include the primary and secondary colors of the app.
mod color;
/// A module containing extension traits for various types
mod ext_traits;
/// The size enum. This enum is used to specify the size of a component. We
/// currently have:
/// - ExtraExtraLarge
/// - ExtraLarge
/// - Large
/// - Medium
/// - Small
/// - ExtraSmall
/// - ExtraExtraSmall
mod size;
/// The storage module. This module is used to store the state of the app in the
/// local storage.
mod storage;
/// The variant enum. This enum is used to specify the variant of a component
/// and the color variant.
mod variant;

#[cfg(not(target_arch = "wasm32"))]
pub use self::client::*;
pub use self::{color::*, ext_traits::*, size::*, storage::*, variant::*};

/// A module containing constants that are used throughout the application.
pub mod constants {
	use dioxus::prelude::*;
	use semver::Version;

	/// The base URL for the backend
	pub const API_BASE_URL: &str = if cfg!(debug_assertions) {
		"http://localhost:3000"
	} else {
		"https://api.patr.cloud"
	};
	/// The version of the application
	pub const VERSION: Version = macros::version!();
	/// The Number of resources to fetch per page
	pub const RESOURCES_PER_PAGE: usize = 2;
	/// The path to the feather icons sprite
	pub const FEATHER_IMG: Asset = asset!("assets/icons/feather-sprite.svg");
	/// The path to the favicon image
	pub const FAVICON: Asset = asset!("assets/favicon.svg");
	/// The path to the CSS file for the dashboard
	pub const GLOBAL_CSS: Asset = asset!("assets/styles/dashboard.css");
	/// The default debounce time for input fields
	pub const DEFAULT_DEBOUNCE_TIME: f64 = 750.0;
	/// The max wait time for the input field debounce
	pub const MAX_DEBOUNCE_TIME: f64 = 1500.0;
}
