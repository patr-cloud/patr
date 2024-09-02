#[cfg(not(target_arch = "wasm32"))]
mod client;

#[cfg(not(target_arch = "wasm32"))]
pub use self::client::*;

/// A module containing extension traits for various types
mod ext_traits;
mod hooks;
mod routes;
mod storage;

pub use self::{ext_traits::*, hooks::*, routes::*, storage::*};

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
}
