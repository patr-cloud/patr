use argon2::{
	password_hash::{PasswordHasher, PasswordVerifier, SaltString},
	Argon2,
	PasswordHash,
	Version,
};
use once_cell::sync::OnceCell;
use sqlx::{MySql, Transaction};
use uuid::Uuid;

use crate::{
	db,
	utils::{settings::Settings, Error},
};

lazy_static::lazy_static! {
	static ref ARGON: Argon2<'static> = Argon2::new(
		None,
		4,
		8192,
		4,
		Version::V0x13
	).unwrap();
}
static APP_SETTINGS: OnceCell<Settings> = OnceCell::new();

pub fn initialize(config: &Settings) {
	APP_SETTINGS
		.set(config.clone())
		.expect("unable to set app settings")
}

pub fn validate_hash(pwd: &str, hashed: &str) -> Result<bool, Error> {
	Ok(ARGON
		.verify_password(
			pwd.as_bytes(),
			&PasswordHash::new(hashed).map_err(|_| Error::empty())?,
		)
		.is_ok())
}

/// function to get token hash
pub fn hash(pwd: &[u8]) -> Result<String, Error> {
	let salt = format!(
		"{}{}",
		APP_SETTINGS
			.get()
			.expect("unable to get app settings")
			.password_pepper,
		SaltString::generate(&mut rand::thread_rng()).as_str()
	);
	ARGON
		.hash_password_simple(pwd, &salt)
		.map(|hash| hash.to_string())
		.map_err(|_| Error::empty())
}

// 2 hours
pub fn get_join_token_expiry() -> u64 {
	1000 * 60 * 60 * 2
}

// 3 days
pub fn get_access_token_expiry() -> u64 {
	1000 * 60 * 60 * 24 * 3
}

// 30 days
pub fn get_refresh_token_expiry() -> u64 {
	1000 * 60 * 60 * 24 * 30
}

// check if the hash is not in the database
pub async fn generate_new_refresh_token_for_user(
	connection: &mut Transaction<'_, MySql>,
	user_id: &[u8],
) -> Result<String, Error> {
	let mut refresh_token = hex::encode(Uuid::new_v4().as_bytes());
	let mut logins = db::get_all_logins_for_user(connection, user_id).await?;
	let mut login = logins.iter().find(|login| {
		if let Ok(result) = validate_hash(&refresh_token, &login.refresh_token)
		{
			result
		} else {
			false
		}
	});

	while login.is_some() {
		refresh_token = hex::encode(Uuid::new_v4().as_bytes());
		logins = db::get_all_logins_for_user(connection, user_id).await?;
		login = logins.iter().find(|login| {
			if let Ok(result) =
				validate_hash(&refresh_token, &login.refresh_token)
			{
				result
			} else {
				false
			}
		});
	}

	Ok(refresh_token)
}

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

#[cfg(feature = "sample-data")]
pub fn generate_new_otp() -> String {
	format!("000000")
}
