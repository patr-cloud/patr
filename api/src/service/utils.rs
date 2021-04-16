use argon2::Variant;
use eve_rs::AsError;

use crate::utils::Error;

pub fn validate_hash(
	pwd: &[u8],
	salt: &[u8],
	hashed: &[u8],
) -> Result<bool, Error> {
	argon2::verify_raw(pwd, salt, hashed, &get_hash_config()).status(500)
}

/// function to get token hash
pub fn hash(pwd: &[u8], salt: &[u8]) -> Result<Vec<u8>, Error> {
	argon2::hash_raw(pwd, salt, &get_hash_config()).status(500)
}

// 2 hours
pub fn get_join_token_expiry() -> u64 {
	1000 * 60 * 60 * 2
}

pub fn get_access_token_expiry() -> u64 {
	0
}

pub fn get_hash_config() -> argon2::Config<'static> {
	argon2::Config {
		variant: Variant::Argon2i,
		hash_length: 64,
		..Default::default()
	}
}

#[cfg(not(feature = "sample-data"))]
pub fn generate_new_otp() -> String {
	use rand::Rng;

	let otp: u32 = rand::thread_rng().gen_range(0, 1_000_000);

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
