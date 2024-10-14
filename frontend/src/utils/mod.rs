#[cfg(not(target_arch = "wasm32"))]
mod client;

#[cfg(not(target_arch = "wasm32"))]
pub use self::client::*;

/// The alignment enum. This enum is used to specify the alignment of a
/// component of left, right, or center.
mod alignment;
/// A module containing the custom [`AppRoute`].
/// The App Route Enum. This Enum is used to specify the route of the app.
mod app_route;
/// The color enum. This enum is used to specify the color of a component. These
/// include the primary and secondary colors of the app.
mod color;
/// A module containing extension traits for various types
mod ext_traits;
mod hooks;
mod routes;
mod sidebar_items;
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
mod storage;
/// The variant enum. This enum is used to specify the variant of a component
/// and the color variant.
mod variant;

pub use self::{
	alignment::*,
	app_route::*,
	color::*,
	ext_traits::*,
	hooks::*,
	routes::*,
	sidebar_items::*,
	size::*,
	storage::*,
	variant::*,
};

/// A trait to extend the [`String`] type with some useful methods that are not
/// available in the standard library. This is useful for adding utility methods
/// to the [`String`] type without polluting the global namespace.
pub trait StringExt {
	/// Wraps the [`String`] into an option depending on whether it's empty
	/// Returns [`None`] if string is empty otherwise returns the string wrapped
	/// in a [`Some()`]
	fn some_if_not_empty(self) -> Option<String>;
}

impl StringExt for String {
	fn some_if_not_empty(self) -> Option<String> {
		if self.is_empty() {
			None
		} else {
			Some(self)
		}
	}
}

/// A module containing constants that are used throughout the application.
pub mod constants {
	use semver::Version;

	/// The version of the application
	pub const VERSION: Version = macros::version!();
	/// The name of the cookie that stores the auth state
	pub const AUTH_STATE: &str = "authState";
	/// The Number of resources to fetch per page
	pub const RESOURCES_PER_PAGE: usize = 2;
	/// The path to the feather icons sprite
	pub const FEATHER_IMG: &str = "/icons/sprite/feather-sprite.svg";
}
