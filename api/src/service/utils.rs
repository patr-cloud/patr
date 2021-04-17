use argon2::{Argon2, Version};
use eve_rs::AsError;
use sqlx::{MySql, Transaction};
use uuid::Uuid;

use crate::{db, utils::Error};

lazy_static::lazy_static! {
	static ref ARGON_CONFIG: Argon2<'static> = Argon2::new(
		secret,
		t_cost,
		m_cost,
		parallelism,
		Version::default()
	).unwrap();
}

pub fn validate_hash(pwd: &str, hashed: &str) -> Result<bool, Error> {
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
) -> Result<Uuid, Error> {
	let mut uuid = Uuid::new_v4();
	let logins = db::get_all_logins_for_user(connection, user_id).await?;

	let mut login = query_as!(
		UserLogin,
		r#"
		SELECT
			*
		FROM
			user_login
		WHERE
			user_id = ?;
		"#,
		user_id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.filter_map(|login| {
		// TODO check salt and pepper for hashes
		if let Ok(same) = service::validate_hash(
			uuid.as_bytes(),
			b"salt",
			&login.refresh_token,
		) {
			Some((same, login))
		} else {
			None
		}
	})
	.find(|(same, login)| *same);

	while login.is_some() {
		uuid = Uuid::new_v4();
		login = query_as!(
			UserLogin,
			r#"
			SELECT
				*
			FROM
				user_login
			WHERE
				login_id = ?;
			"#,
			uuid.as_bytes().as_ref()
		)
		.fetch_all(&mut *connection)
		.await?
		.into_iter()
		.filter_map(|login| {
			// TODO check salt and pepper for hashes
			service::validate_hash(
				uuid.as_bytes(),
				b"salt",
				&login.refresh_token,
			)
			.ok()
		})
		.find(|login| *login);
	}

	Ok(uuid)
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
