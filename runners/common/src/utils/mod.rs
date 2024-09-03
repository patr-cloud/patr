/// The client for the Patr API to get runner data for a given workspace.
pub mod client;
/// The configuration for the runner.
pub mod config;
/// A utility that returns a value after a delay.
pub mod delayed_future;
/// Extensions traits for the `Either` type.
pub mod ext_traits;

/// The constants module contains all the constants that are used throughout
/// the runner Project.
pub mod constants {
	use semver::Version;

	/// The version of the database. This is used to determine whether the
	/// database needs to be migrated or not. This is always set to the manifest
	/// version in Cargo.toml.
	pub const DATABASE_VERSION: Version = macros::version!();
}
