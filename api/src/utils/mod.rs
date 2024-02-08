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

mod router_ext;

pub use self::router_ext::RouterExt;

/// A list of constants that will be used throughout the application. This is
/// mostly kept to prevent typos.
pub mod constants {
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
	pub const ACCESS_TOKEN_VALIDITY: time::Duration = time::Duration::hours(1);

	/// The version of the database. This is used to determine whether the
	/// database needs to be migrated or not. This is always set to the manifest
	/// version in Cargo.toml.
	pub const DATABASE_VERSION: Version = macros::version!();

	/// The channel to publish and listen for data on from the database. This is
	/// used to notify the backend when data has changed in the database, so
	/// that it can notify the frontend via websockets.
	pub const DATABASE_CHANNEL: &str = "data";
}
