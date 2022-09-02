use api_models::{
	models::workspace::infrastructure::deployment::DeploymentRegistry,
	utils::Uuid,
};
use argon2::{
	password_hash::{PasswordVerifier, SaltString},
	Algorithm,
	Argon2,
	Params,
	PasswordHash,
	PasswordHasher,
	Version,
};
use chrono::Duration;
use eve_rs::AsError;

use crate::{
	db,
	error,
	service,
	utils::{settings::Settings, validator, Error},
	Database,
};

lazy_static::lazy_static! {
	static ref ARGON: Argon2<'static> = Argon2::new(
		Algorithm::Argon2id,
		Version::V0x13,
		Params::new(8192, 4, 4, None).unwrap(),
	);
}

/// # Description
/// This function is used to validate the hashed password using [`static@ARGON`]
///
/// # Arguments
/// * `pwd` - a string containing password of the user
/// * `hashed` - a string containing hashed password of the user
///
/// # Returns
/// This function returns `Result<bool, Error>` containing bool stating the the
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
/// This function is used to get token hash
///
/// # Arguments
/// * `pwd` - an unsigned 8 bit integer array containing hashed password of the
/// user
///
/// # Returns
/// This function returns `Result<String, Error>` containing hashed password or
/// an error
pub fn hash(pwd: &[u8]) -> Result<String, Error> {
	let salt = format!(
		"{}{}",
		service::get_settings().password_pepper,
		SaltString::generate(&mut rand::thread_rng()).as_str()
	);
	ARGON
		.hash_password(pwd, &salt)
		.map(|hash| hash.to_string())
		.map_err(|_| Error::empty())
}

/// # Description
/// This function is used to get join token expiry in millisec.
/// set to: `2 hours`
///
/// # Returns
/// This function returns unsigned 64 bit integer (unix time)
pub fn get_join_token_expiry() -> Duration {
	Duration::hours(2)
}

/// # Description
/// This function is used to get access token expiry in millisec.
/// set to: `3 days`
///
/// # Returns
/// This function returns unsigned 64 bit integer (unix time)
pub fn get_access_token_expiry() -> Duration {
	Duration::days(3)
}

/// # Description
/// This function is used to get refresh token expiry in millisec.
/// set to: `30 days`
///
/// # Returns
/// This function returns unsigned 64 bit integer (unix time)
pub fn get_refresh_token_expiry() -> Duration {
	Duration::days(30)
}

/// # Description
/// This function is used to generate a new refresh token and check if the token
/// is already present or not in the database
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `user_id` - an unsigned 8 bit integer array containing id of user
///
/// # Returns
/// This function returns Result<(String, String), Error> containing strings
/// refresh token and hashed form of it
///
/// [`Transaction`]: Transaction
pub async fn generate_new_refresh_token_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<(Uuid, String), Error> {
	loop {
		let refresh_token = Uuid::new_v4();
		let hashed = hash(refresh_token.as_str().as_bytes())?;
		let login = db::get_login_for_user_with_refresh_token(
			connection, user_id, &hashed,
		)
		.await?;

		if login.is_none() {
			// No login exists with that refresh token. Break
			break Ok((refresh_token, hashed));
		}
	}
}

#[cfg(all(feature = "sample-data", release))]
pub fn sample_data_not_allowed_in_release_mode() {
	compile_error!("Populating sample data is not allowed in release mode");
}

/// # Description
/// this function is used to generate a new otp, but for development purpose
/// this function will not be used, it will only be used in production or
/// testing
///
/// # Returns
/// returns a string containing a 6 digit One-Time-Password
#[cfg(not(debug_assertions))]
pub fn generate_new_otp() -> String {
	use rand::Rng;

	let otp: u32 = rand::thread_rng().gen_range(0..1_000_000);

	let otp = if otp < 10 {
		format!("00000{}", otp)
	} else if otp < 100 {
		format!("0000{}", otp)
	} else if otp < 1000 {
		format!("000{}", otp)
	} else if otp < 10000 {
		format!("00{}", otp)
	} else if otp < 100_000 {
		format!("0{}", otp)
	} else {
		format!("{}", otp)
	};
	format!("{}-{}", &otp[..3], &otp[3..])
}

/// # Description
/// this function is used to generate a new otp only during the development time
///
/// # Returns
/// returns a string containing a 6 digit One-Time-Password (000000)
#[cfg(debug_assertions)]
pub fn generate_new_otp() -> String {
	"000-000".to_string()
}

/// # Description
/// this function is used to split the email into local and domain
/// and extract the domain id from it
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `email_address` - a string containing the email address to be split
///
/// # Returns
/// this function returns a `Result<(String, Vec<u8>)>` either containing a
/// tuple of string of email local and an unsigned 8 bit integer Vec containing
/// domain id or an error
///
/// [`Transaction`]: Transaction
pub async fn split_email_with_domain_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	email_address: &str,
) -> Result<(String, Uuid), Error> {
	let (email_local, domain_name) = email_address
		.split_once('@')
		.status(400)
		.body(error!(INVALID_EMAIL).to_string())?;

	let domain_id =
		service::ensure_personal_domain_exists(connection, domain_name).await?;

	Ok((email_local.to_string(), domain_id))
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

pub async fn split_domain_and_tld(
	domain_name: &str,
) -> Option<(String, String)> {
	let tld_list = validator::DOMAIN_TLD_LIST.read().await;
	let tld = tld_list
		.iter()
		.filter(|tld| domain_name.ends_with(*tld))
		.reduce(|accumulator, item| {
			if accumulator.len() > item.len() {
				accumulator
			} else {
				item
			}
		})?;
	let domain = domain_name.replace(&format!(".{}", tld), "");
	// domain cannot begin or end with a -, and must be at least 1 character
	if domain.contains('.') ||
		domain.is_empty() ||
		domain.starts_with('-') ||
		domain.ends_with('-') ||
		domain
			.chars()
			.any(|c| !c.is_ascii_alphanumeric() && c != '-')
	{
		return None;
	}

	Some((domain, tld.clone()))
}

pub async fn get_image_name_and_digest_for_deployment_image(
	connection: &mut <Database as sqlx::Database>::Connection,
	registry: &DeploymentRegistry,
	image_tag: &str,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(String, Option<String>), Error> {
	match registry {
		DeploymentRegistry::PatrRegistry {
			registry: _,
			repository_id,
		} => {
			log::trace!(
				"request_id: {} - Getting tag details from database",
				request_id
			);
			let digest = db::get_docker_repository_tag_details(
				connection,
				repository_id,
				image_tag,
			)
			.await?
			.map(|(_, digest)| digest);

			log::trace!(
				"request_id: {} - Getting repository details from the database",
				request_id
			);
			let repository_details =
				db::get_docker_repository_by_id(connection, repository_id)
					.await?
					.status(500)?;

			log::trace!(
				"request_id: {} - Getting workspace details from the database",
				request_id
			);
			let workspace_id = db::get_workspace_info(
				connection,
				&repository_details.workspace_id,
			)
			.await?
			.status(500)?
			.id;

			Ok((
				format!(
					"{}/{}/{}",
					config.docker_registry.registry_url,
					workspace_id,
					repository_details.name
				),
				digest,
			))
		}
		DeploymentRegistry::ExternalRegistry {
			registry,
			image_name,
		} => match registry.as_str() {
			"registry.hub.docker.com" |
			"hub.docker.com" |
			"index.docker.io" |
			"docker.io" |
			"" => Ok((format!("docker.io/{}", image_name), None)),
			_ => Ok((format!("{}/{}", registry, image_name), None)),
		},
	}
}
