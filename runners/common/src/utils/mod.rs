/// The data that is stored inside the access token, which will be encoded as a
/// JWT.
pub mod access_token_data;
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
	/// The issuer (iss) of the JWT. This is currently the URL of Patr API.
	pub const JWT_ISSUER: &str = "https://api.patr.cloud";
	/// The parameters that will be used to hash, using argon2 as the hashing
	/// algorithm. This is used for all sorts of hashing, from API tokens, user
	/// passwords, sign up tokens, etc.
	pub const HASHING_PARAMS: argon2::Params =
		if let Ok(params) = argon2::Params::new(8192, 4, 4, None) {
			params
		} else {
			panic!("Failed to create hashing params");
		};
	/// The audience (aud) of the JWT. This is currently set to "patr.cloud".
	pub const PATR_JWT_AUDIENCE: &str = "patr.cloud";
	/// The expiry time for the access token. This is set to 7 days.
	pub const ACCESS_TOKEN_VALIDITY: time::Duration = time::Duration::days(7);
	/// The user_id key to be useed in the meta_data table. This is used to
	/// store the user_id of the user that is currently logged in.
	pub const USER_ID_KEY: &str = "user_id";
	/// The Password Hash key to be used in the meta_data table. This is used to
	/// store the password hash of the user that is currently logged in.
	pub const PASSWORD_HASH_KEY: &str = "password_hash";
	/// The First Name key to be used in the meta_data table. This is used to
	/// store the first name of the user that is currently logged in.
	pub const FIRST_NAME_KEY: &str = "first_name";
	/// The Last Name key to be used in the meta_data table. This is used to
	/// store the last name of the user that is currently logged in.
	pub const LAST_NAME_KEY: &str = "last_name";
}
