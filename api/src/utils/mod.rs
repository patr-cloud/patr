/// The configuration data that is loaded when the backend starts. This contains
/// the details of the database, Redis, the JWT secret, etc.
pub mod config;
/// Contains the [`layer`][1]s that will be used with [`tower`] mounted on the
/// axum [`Router`][2]
///
/// [1]: tower::Layer
/// [2]: axum::Router
pub mod layers;

/// Contains the [`extractor`][1]s that will be used with [`tower`] mounted on
/// the axum [`Router`][2]
///
/// [1]: axum::extract::FromRequest
/// [2]: axum::Router
pub mod extractors;

/// Contains the extension traits that will be used with the axum [`Router`][1]
/// to mount the various endpoints on the router.
///
/// [1]: axum::Router
mod router_ext;

/// Contains the extension traits that will be used to timeout futures as
/// they're executing.
mod timeout_ext;

pub use self::{router_ext::RouterExt, timeout_ext::TimeoutExt};

/// A list of constants that will be used throughout the application. This is
/// mostly kept to prevent typos.
pub mod constants {
	use std::ops::RangeInclusive;

	use semver::Version;

	/// The issuer (iss) of the JWT. This is currently the URL of Patr API.
	pub const JWT_ISSUER: &str = "https://api.patr.cloud";

	/// The `aud` field in Patr's JWT
	pub const PATR_JWT_AUDIENCE: &str = "https://app.patr.cloud";

	/// The parameters that will be used to hash, using argon2 as the hashing
	/// algorithm. This is used for all sorts of hashing, from API tokens, user
	/// passwords, sign up tokens, etc.
	pub const HASHING_PARAMS: argon2::Params =
		if let Ok(params) = argon2::Params::new(8192, 4, 4, None) {
			params
		} else {
			panic!("Failed to create hashing params");
		};

	/// How long a refresh token, once generated, is valid for without any
	/// activity. After this duration of no activity on the refresh token, it
	/// will be considered expired.
	pub const INACTIVE_REFRESH_TOKEN_VALIDITY: time::Duration = time::Duration::days(30);

	/// How long an access token is valid before it needs to be refreshed using
	/// a refresh token (which will be provided at login)
	pub const ACCESS_TOKEN_VALIDITY: time::Duration = if cfg!(debug_assertions) {
		time::Duration::weeks(52) // 1 year
	} else {
		time::Duration::hours(1)
	};

	/// The duration that the permission data in Redis will be valid for. Beyond
	/// that, the data will be considered stale and will be reloaded from the
	/// database. This is done to prevent the Redis data from having infinite
	/// keys for permission revocations, since they're not stored in the
	/// database.
	pub const CACHED_PERMISSIONS_VALIDITY: time::Duration = time::Duration::days(2);

	/// The version of the database. This is used to determine whether the
	/// database needs to be migrated or not. This is always set to the manifest
	/// version in Cargo.toml.
	pub const DATABASE_VERSION: Version = macros::version!();

	/// The channel to publish and listen for data on from the database. This is
	/// used to notify the backend when data has changed in the database, so
	/// that it can notify the frontend via websockets.
	pub const DATABASE_CHANNEL: &str = "data";

	/// The range within which to randomly generate an OTP
	pub const OTP_RANGE: RangeInclusive<u64> = if cfg!(debug_assertions) {
		RangeInclusive::new(0, 0)
	} else {
		RangeInclusive::new(0, 999_999)
	};

	/// How long an OTP is valid for. After this time, the OTP will be invalid
	/// and the error returned will be the same as an "OTP doesn't exist" error
	/// to prevent it from leaking old OTPs.
	pub const OTP_VALIDITY: time::Duration = time::Duration::hours(2);

	/// The default maximum limit for the number of workspaces a user can
	/// create. If this needs to be increased, the user should open a support
	/// ticket with the team.
	pub const DEFAULT_WORKSPACE_LIMIT: i32 = 10;

	/// The maximum number of times a user can attempt to reset a password
	/// before getting banned altogether
	pub const MAX_PASSWORD_RESET_ATTEMPTS: u16 = 5;
}
