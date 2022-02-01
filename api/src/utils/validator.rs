use lazy_static::lazy_static;
use regex::Regex;
use tokio::sync::RwLock;

lazy_static! {
	// List of all TLDs supported by ICANN. Updated every week.
	pub static ref DOMAIN_TLD_LIST: RwLock<Vec<String>> = RwLock::new(vec![]);
	// Can only contain a-z, A-Z, 0-9, . and _. Cannot begin with a . (github rules, basically) and at least two characters.
	static ref USERNAME_REGEX: Regex = Regex::new("^[a-z0-9_][a-z0-9_\\.\\-]*[a-z0-9_]$").unwrap();
	// Email regex: https://stackoverflow.com/a/201378
	static ref EMAIL_REGEX: Regex = Regex::new("^(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*|\"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*\")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\\[(?:(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9]))\\.){3}(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9])|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\\])$").unwrap();
	// Needs to have a '+' to start with, and be between 9-15 numbers after that
	static ref PHONE_NUMBER_REGEX: Regex = Regex::new("^[0-9]{7,15}$").unwrap();
	// Needs to have at least 1 a-z, 1 A-Z, 1 0-9 and a special character
	//static ref PASSWORD_REGEX: Regex = Regex::new("^(?=.*[0-9])(?=.*[a-z])(?=.*[A-Z])(?=.*[@#$%^&-+=()])(?=\\S+$).{8,}$").unwrap();
	// Needs to begin with personal-workspace- and follow up with a 128 bit hex
	static ref PERSONAL_WORKSPACE_NAME_REGEX: Regex = Regex::new("^personal-workspace-[a-z0-9]{32}$").unwrap();
	// Can only contain lowercase letters, numbers, hyphens and underscores
	static ref DOCKER_REPO_NAME_REGEX: Regex = Regex::new("^[a-z0-9_-]{2,255}$").unwrap();
	// Validate the name of database
	static ref DATABASE_NAME_REGEX: Regex = Regex::new("^[a-zA-Z][a-zA-Z0-9_]{2,59}$").unwrap();
	// 2-64 characters long ([a-zA-Z0-9_- .]), cannot begin with a _, -, . or a space, cannot end with a space
	static ref DEPLOYMENT_NAME_REGEX: Regex = Regex::new("^[a-zA-Z0-9_\\-\\.][a-zA-Z0-9_\\-\\. ]{0,62}[a-zA-Z0-9_\\-\\.]$").unwrap();
}

pub fn is_username_valid(username: &str) -> bool {
	username.len() <= 100 &&
		USERNAME_REGEX.is_match(username) &&
		!username.contains("..") &&
		!username.contains("--") &&
		!username.contains(".-") &&
		!username.contains("-.")
}

pub fn is_email_valid(email: &str) -> bool {
	email.len() <= 320 && EMAIL_REGEX.is_match(email)
}

pub fn is_password_valid(password: &str) -> bool {
	let mut has_lower_case = false;
	let mut has_upper_case = false;
	let mut has_number = false;
	let mut has_special_character = false;
	password.chars().for_each(|ch| {
		if ch.is_ascii_lowercase() {
			has_lower_case = true;
		}
		if ch.is_ascii_uppercase() {
			has_upper_case = true;
		}
		if ch.is_numeric() {
			has_number = true;
		}
		if "~`!@#$%^&*()-_+=[]{};':\",./<>?".contains(ch) {
			has_special_character = true;
		}
	});
	password.len() >= 8 &&
		has_lower_case &&
		has_upper_case &&
		has_number &&
		has_special_character
}

pub fn is_phone_number_valid(phone_number: &str) -> bool {
	PHONE_NUMBER_REGEX.is_match(phone_number)
}

pub fn is_workspace_name_valid(workspace_name: &str) -> bool {
	!PERSONAL_WORKSPACE_NAME_REGEX.is_match(workspace_name)
}

pub fn is_docker_repo_name_valid(repo_name: &str) -> bool {
	DOCKER_REPO_NAME_REGEX.is_match(repo_name)
}

pub fn is_deployment_name_valid(deployment_name: &str) -> bool {
	DEPLOYMENT_NAME_REGEX.is_match(deployment_name)
}

pub fn is_database_name_valid(database_name: &str) -> bool {
	database_name.len() <= 64 && DATABASE_NAME_REGEX.is_match(database_name)
}
