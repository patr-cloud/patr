/// The data that is stored inside the access token, which will be encoded as a
/// JWT.
pub mod access_token_data;
/// The assets that will be embedded in the binary. This is used to serve the
/// assets for the runner.
pub mod assets;
/// The client for the Patr API to get runner data for a given workspace.
pub mod client;
/// The configuration for the runner.
pub mod config;
/// Extensions traits for the `Either` type.
pub mod ext_traits;

/// Contains the [`layer`][1]s that will be used with [`tower`] mounted on the
/// axum [`Router`][2]
///
/// [1]: tower::Layer
/// [2]: axum::Router
mod layers;
/// Contains the extension traits that will be used with the axum [`Router`][1]
/// to mount the various endpoints on the router.
///
/// [1]: axum::Router
mod router_ext;
/// The type of exposure that the runner will use to expose the resources.
mod runner_exposure_type;

use sqlx::sqlite::{SqliteOperation, UpdateHookResult};

pub use self::{router_ext::*, runner_exposure_type::*};

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
	/// The `user_id` key to be used in the `meta_data` table. This is used to
	/// store the `user_id` of the user that is currently logged in.
	pub const USER_ID_KEY: &str = "user_id";
	/// The Password Hash key to be used in the `meta_data` table. This is used
	/// to store the password hash of the user that is currently logged in.
	pub const PASSWORD_HASH_KEY: &str = "password_hash";
	/// The First Name key to be used in the `meta_data` table. This is used to
	/// store the first name of the user that is currently logged in.
	pub const FIRST_NAME_KEY: &str = "first_name";
	/// The Last Name key to be used in the `meta_data` table. This is used to
	/// store the last name of the user that is currently logged in.
	pub const LAST_NAME_KEY: &str = "last_name";

	// NGINX related constants

	/// The nginx configuration file path. This is used when the runner does not
	/// support it's own URL mechanism and needs to use nginx to
	/// serve the runner deployments.
	pub const NGINX_CONFIG_PATH: &str = "./data/nginx/nginx.conf";
	/// The socket file path for nginx to bind to. This is where nginx will
	/// listen for connections on and the cloudflare tunnel should send traffic
	/// to this path.
	pub const NGINX_SOCKET_PATH: &str = "./data/nginx/nginx.sock";
	/// The nginx lock file path. This is used to ensure that only one instance
	/// of nginx is running at a time. When nginx starts, it will bind to
	/// [`NGINX_SOCKET_PATH`]. But if the file already exists, nginx will cry.
	/// But we can't simply delete the file before starting either, because in
	/// case any other instance is running, we cannot delete the file for that
	/// instance. So we create a lock file at this path, and acquire an OS lock
	/// on it. If we have acquired the lock, we know for a fact that we are the
	/// only instance running (because otherwise the other instance would've
	/// acquired the lock). So we can safely delete the socket file and start
	/// nginx.
	pub const NGINX_LOCK_FILE_PATH: &str = "./data/nginx/nginx.lock";
}

/// The data that is returned by the SQLite update hook. This contains the same
/// fields as [`UpdateHookResult`][1] but has owned strings for being able to
/// send it across threads and tasks.
///
/// [1]: sqlx::sqlite::UpdateHookResult
#[derive(Debug, Clone)]
pub struct SqliteUpdateHook {
	/// The operation that was performed on the database.
	pub operation: SqliteOperation,
	/// The database that was modified.
	pub database: String,
	/// The table that was modified.
	pub table: String,
	/// The row ID of the modified row.
	pub row_id: i64,
}

impl From<UpdateHookResult<'_>> for SqliteUpdateHook {
	fn from(value: UpdateHookResult<'_>) -> Self {
		Self {
			operation: value.operation,
			database: value.database.to_string(),
			table: value.table.to_string(),
			row_id: value.rowid,
		}
	}
}
