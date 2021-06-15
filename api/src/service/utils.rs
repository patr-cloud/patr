use argon2::{
	password_hash::{PasswordHasher, PasswordVerifier, SaltString},
	Argon2,
	PasswordHash,
	Version,
};
use hex::ToHex;
use uuid::Uuid;

use crate::{db, service, utils::Error, Database};

lazy_static::lazy_static! {
	static ref ARGON: Argon2<'static> = Argon2::new(
		None,
		4,
		8192,
		4,
		Version::V0x13
	).unwrap();
}

/// # Description
///	this function is used to validate the hashed password using [`static@ARGON`]
///
/// # Arguments
/// * `pwd` - a string containing password of the user
/// * `hashed` - a string containing hashed password of the user
///
/// # Returns
///	this function returns `Result<bool, Error>` containing bool stating the the
/// password is successfully validated or not or an error
/// [`static@ARGON`]: static@ARGON
pub fn validate_hash(pwd: &str, hashed: &str) -> Result<bool, Error> {
	Ok(ARGON
		.verify_password(
			pwd.as_bytes(),
			&PasswordHash::new(hashed).map_err(|_| Error::empty())?,
		)
		.is_ok())
}

/// # Description
/// this function is used to get token hash
///
/// # Arguments
/// * `pwd` - an unsigned 8 bit integer array containing hashed password of the
/// user
///
/// # Returns
///	this function returns Result<String, Error> containing hashed password or an
/// error
pub fn hash(pwd: &[u8]) -> Result<String, Error> {
	let salt = format!(
		"{}{}",
		service::get_config().password_pepper,
		SaltString::generate(&mut rand::thread_rng()).as_str()
	);
	ARGON
		.hash_password_simple(pwd, &salt)
		.map(|hash| hash.to_string())
		.map_err(|_| Error::empty())
}

/// # Description
///	function to get join token expiry
/// set to: `2 hours`
///
/// # Returns
///	this function returns unsigned 64 bit integer (unix time)
pub fn get_join_token_expiry() -> u64 {
	1000 * 60 * 60 * 2
}

/// # Description
///	function to get access token expiry
/// set to: `3 days`
///
/// # Returns
///	this function returns unsigned 64 bit integer (unix time)
pub fn get_access_token_expiry() -> u64 {
	1000 * 60 * 60 * 24 * 3
}

/// # Description
///	function to get refresh token expiry
/// set to: `30 days`
///
/// # Returns
///	this function returns unsigned 64 bit integer (unix time)
pub fn get_refresh_token_expiry() -> u64 {
	1000 * 60 * 60 * 24 * 30
}

/// # Description
/// this function is used to generate a new refresh token and check if the token
/// is already present or not in the database
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `user_id` - an unsigned 8 bit integer array containing id of user
///
/// # Returns
/// this function returns Result<(String, String), Error> containing strings
/// refresh token and hashed form of it
pub async fn generate_new_refresh_token_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8],
) -> Result<(String, String), Error> {
	let mut refresh_token = Uuid::new_v4().as_bytes().encode_hex::<String>();
	let mut hashed = hash(refresh_token.as_bytes())?;
	let mut logins = db::get_all_logins_for_user(connection, user_id).await?;
	let mut login = logins.iter().find(|login| login.refresh_token == hashed);

	while login.is_some() {
		refresh_token = Uuid::new_v4().as_bytes().encode_hex();
		hashed = hash(refresh_token.as_bytes())?;
		logins = db::get_all_logins_for_user(connection, user_id).await?;
		login = logins.iter().find(|login| login.refresh_token == hashed);
	}

	Ok((refresh_token, hashed))
}

/// # Description
/// this function is used to generate a new otp, but for development purpose
/// this function will not be used, it will only be used in production or
/// testing
///
/// # Returns
/// returns a string containing a 6 digit One-Time-Password
#[cfg(not(feature = "sample-data"))]
pub fn generate_new_otp() -> String {
	use rand::Rng;

	let otp: u32 = rand::thread_rng().gen_range(0..1_000_000);

	if otp < 10 {
		format!("00000{}", otp)
	} else if otp < 100 {
		format!("0000{}", otp)
	} else if otp < 1000 {
		format!("000{}", otp)
	} else if otp < 10000 {
		format!("00{}", otp)
	} else if otp < 100000 {
		format!("0{}", otp)
	} else {
		format!("{}", otp)
	}
}

/// # Description
/// this function is used to generate a new otp only during the development time
///
/// # Returns
/// returns a string containing a 6 digit One-Time-Password (000000)
#[cfg(feature = "sample-data")]
pub fn generate_new_otp() -> String {
	"000000".to_string()
}

/// # Description
/// this function is used to mask the email local by "*" character
///
/// # Arguments
/// * `local` - a string containing email_local
///
/// # Returns
/// this function returns a `String` containg masked email local
pub fn mask_email_local(local: &str) -> String {
	if local.is_empty() {
		String::from("*")
	} else if local.len() <= 2 {
		local.chars().map(|_| '*').collect()
	} else if local.len() > 2 && local.len() <= 4 {
		if let Some(first) = local.chars().next() {
			format!("{}***", first)
		} else {
			String::from("*")
		}
	} else {
		let mut chars = local.chars();
		let first = if let Some(first) = chars.next() {
			first
		} else {
			return String::from("*");
		};
		let last = if let Some(last) = chars.last() {
			last
		} else {
			return String::from("*");
		};
		format!(
			"{}{}{}",
			first,
			if local.len() < 10 { "****" } else { "********" },
			last
		)
	}
}

/// # Description
/// this function is used to mask the phone number by "*" character
///
/// # Arguments
/// * `phone_number` - a string containing phone_number
///
/// # Returns
/// this function returns a `String` containg masked phone number
pub fn mask_phone_number(phone_number: &str) -> String {
	let mut chars = phone_number.chars();
	let first = if let Some(first) = chars.next() {
		first
	} else {
		return String::from("******");
	};
	let last = if let Some(last) = chars.last() {
		last
	} else {
		return String::from("******");
	};
	format!("{}******{}", first, last)
}
