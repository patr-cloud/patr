/// Contains the asset handling for emails, error pages, and other static files.
/// This includes the embedded assets, the S3 upload logic, and the functions to
/// get the URLs for the assets.
pub mod assets;
/// Cloudflare utilities: ingress KV sync, tunnel config, and Turnstile
/// validation.
pub mod cloudflare;
/// The configuration data that is loaded when the backend starts. This contains
/// the details of the database, Redis, the JWT secret, etc.
pub mod config;
/// Contains the [`extractor`][1]s that will be used with [`tower`] mounted on
/// the axum [`Router`][2]
///
/// [1]: axum::extract::FromRequest
/// [2]: axum::Router
pub mod extractors;
/// Contains the [`layer`][1]s that will be used with [`tower`] mounted on the
/// axum [`Router`][2]
///
/// [1]: tower::Layer
/// [2]: axum::Router
pub mod layers;

/// Contains the extension traits that will be used to add functionality to
/// existing types.
mod extensions;
/// Contains the log formatter for the API. This is used to format the logs in a
/// specific way, and to remove any fields that are not needed from the logs.
mod tracing;

pub use self::{extensions::*, tracing::*};

/// A list of constants that will be used throughout the application. This is
/// mostly kept to prevent typos.
pub mod constants {
	use std::{ops::RangeInclusive, time::Duration};

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
	pub const OTP_VALIDITY: time::Duration = time::Duration::minutes(15);

	/// The default maximum limit for the number of workspaces a user can
	/// create. If this needs to be increased, the user should open a support
	/// ticket with the team.
	pub const DEFAULT_WORKSPACE_LIMIT: i32 = 10;

	/// The maximum number of times a user can attempt to reset a password
	/// before getting banned altogether
	pub const MAX_PASSWORD_RESET_ATTEMPTS: i32 = 5;

	/// The issuer to be used for TOTP generation
	pub const TOTP_ISSUER: &str = "app.patr.cloud";

	/// The validity duration for an IP lookup. This is the duration for which
	/// the IP lookup data will be stored in Redis and considered valid. After
	/// this duration, the data will be considered stale and will be deleted
	/// from Redis, and a new lookup will be performed the next time an IP
	/// lookup is needed.
	pub const IP_LOOKUP_SUCCESS_VALIDITY: time::Duration = time::Duration::days(7);

	/// The validity duration for a failed IP lookup. This is the duration for
	/// which the failed IP lookup data will be stored in Redis and considered
	/// valid. After this duration, the data will be considered stale and will
	/// be deleted from Redis, and a new lookup will be performed the next time
	/// an IP lookup is needed. This is kept separate from the success validity
	/// duration to prevent the backend from performing repeated lookups for IPs
	/// that are consistently failing, which can help reduce costs and
	/// unnecessary load on the IP lookup service.
	pub const IP_LOOKUP_FAILURE_VALIDITY: time::Duration = time::Duration::days(1);

	// -------------------All Registry Related Constants-------------------

	/// The regex that a registry repository name must conform to
	pub const REGISTRY_REPO_NAME_REGEX: &str = macros::verify_regex!(
		"^[a-z0-9]+((\\.|_|__|-+)[a-z0-9]+)*(\\/[a-z0-9]+((\\.|_|__|-+)[a-z0-9]+)*)*$"
	);

	/// The regex that a registry digest must conform to
	pub const REGISTRY_DIGEST_REGEX: &str =
		macros::verify_regex!("^[A-Za-z][A-Za-z0-9+._-]*:(?:[a-f0-9]{2})+$");

	/// The regex that a registry tag must conform to
	pub const REGISTRY_TAG_REGEX: &str =
		macros::verify_regex!("^[a-zA-Z0-9_][a-zA-Z0-9._-]{0,127}$");

	/// The regex that a registry tag / digest reference must conform to
	pub const REGISTRY_TAG_OR_DIGEST_REGEX: &str = macros::verify_regex!(
		"^(?:[A-Za-z0-9_][A-Za-z0-9._-]{0,127}|[A-Za-z][A-Za-z0-9+._-]*:(?:[a-f0-9]{2})+)$"
	);

	/// The duration for which a registry blob upload session will be valid for.
	/// This is the duration for which the session data will be stored in Redis,
	/// and the duration for which the S3 multipart upload will be valid for.
	/// After this duration, the session will be considered expired and the S3
	/// multipart upload will have to be aborted and cleaned up.
	pub const REGISTRY_BLOB_UPLOAD_SESSION_TTL: Duration = Duration::from_hours(24);

	/// The duration for which the pending buffer of a registry blob upload
	/// session will be valid for. This is the duration for which the pending
	/// buffer data will be stored in Redis. After this duration, the pending
	/// buffer will be considered expired and will be deleted from Redis.
	pub const REGISTRY_BLOB_UPLOAD_PENDING_BUFFER_TTL: Duration = Duration::from_hours(24);

	/// The maximum size of a manifest that can be uploaded to the registry.
	/// This is used to prevent DoS attacks by uploading extremely large
	/// manifests that can consume a lot of memory when being processed. The
	/// value is set to 100 MiB, which should be sufficient for most users,
	/// since the average manifest size is usually in the range of a few KB to a
	/// few MBs, even for large images with many layers. If a user needs to
	/// upload a manifest larger than this size, they can contact support to
	/// have the limit increased for their account.
	pub const MAX_REGISTRY_MANIFEST_SIZE: usize = 100 * 1024 * 1024; // 100 MiB
}
