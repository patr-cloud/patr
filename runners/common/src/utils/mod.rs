/// The client for the Patr API to get runner data for a given workspace.
pub mod client;
/// The configuration for the runner.
pub mod config;
/// OTLP observability setup for managed-mode runners.
pub mod observability;
/// Reading secret values from the Patr-hosted OpenBao proxy.
pub mod secrets;

/// The constants module contains all the constants that are used throughout
/// the runner Project.
pub mod constants {
	use semver::Version;

	/// The version of the database. This is used to determine whether the
	/// database needs to be migrated or not. This is always set to the manifest
	/// version in Cargo.toml.
	pub const DATABASE_VERSION: Version = macros::version!();
}
