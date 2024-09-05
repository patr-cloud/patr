/// The client for the Patr API to get runner data for a given workspace.
pub mod client;
/// The configuration for the runner.
pub mod config;
/// A utility that returns a value after a delay.
pub mod delayed_future;
/// Extensions traits for the `Either` type.
pub mod ext_traits;

/// Contains the extension traits that will be used with the axum [`Router`][1]
/// to mount the various endpoints on the router.
///
/// [1]: axum::Router
mod router_ext;

/// Contains the [`layer`][1]s that will be used with [`tower`] mounted on the
/// axum [`Router`][2]
///
/// [1]: tower::Layer
/// [2]: axum::Router
mod layers;

pub use self::router_ext::RouterExt;

/// The constants module contains all the constants that are used throughout
/// the runner Project.
pub mod constants {
	use semver::Version;

	/// The version of the database. This is used to determine whether the
	/// database needs to be migrated or not. This is always set to the manifest
	/// version in Cargo.toml.
	pub const DATABASE_VERSION: Version = macros::version!();
}
