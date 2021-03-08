pub mod db_mapping;
pub mod error;
pub mod rbac;

mod access_token_data;
mod docker_registry_token;
mod twilio_sms_body;

pub use access_token_data::*;
pub use docker_registry_token::*;
pub use twilio_sms_body::*;

/*
New:

Users belong to an organisation through a role
Users can create an organisation for all their personal resources
Roles have permissions on a resource type or a specific resource
Resources belong to an organisation
Actions on a resource require permissions on that resource


When validating a request:
- Check how the user has access to the resouce.
- If the user has access to the resource directly,
	- Check if their personal roles grant the required permissions
- If the user has access to the resource through an organisation,
	- Check if their organisation roles grant the required permissions


Each resource must be "owned" by someone or the other.
There can't be a resouce that doesn't have an owner.
2 ways to do this:
- The "owner" role cannot be removed from a resource.
  Can be transferred, maybe, but no.
	Pros:
	- Fits in well with rbac middlewares. Can be done in the same SQL query
	Cons:
	- In case, by mistake, the role is removed, we now have a dangling resource
- The database schema for a resource has a "owner" field
  that points to either a user or an org
	Pros:
	- Dangling resources can't exist. They NEED to be owned by someone as per the db schema
	Cons:
	- Need to do a more complex query to check if owner field as a valid role


-------















Resources <- require -> (one or many) Permissions.
Roles <- are collections of -> (one or many) Permissions.
Users <- can have -> (one or many) Roles.

The tables for such a model would be:
permission
role
user
role_permission
user_role

Permission model:

Users:
- UserID
- Roles[]

Organizations:
- Users[]

Roles:
- RoleID
- Permissions[]

Permission:
- PermissionID
- PermissionType

Resources:
- PermissionsRequired[]

*/

#[cfg(feature = "sample-data")]
pub async fn initialize_sample_data(config: crate::app::App) {
	use std::{collections::HashMap, time::Duration};

	use crate::constants::request_keys;
	use serde::Deserialize;
	use serde_json::{json, Value};
	use surf::{http::Method, url::Url, Request};

	#[derive(Debug, Deserialize)]
	#[serde(rename_all = "camelCase")]
	struct SampleDataUser {
		username: String,
		first_name: String,
		last_name: String,
		backup_email: String,
		password: String,
	}

	#[derive(Debug, Deserialize)]
	#[serde(rename_all = "camelCase")]
	struct SampleDataApplication {
		name: String,
		versions: Vec<String>,
	}

	#[derive(Debug, Deserialize)]
	#[serde(rename_all = "camelCase")]
	struct SampleDataOrganisation {
		name: String,
		domains: Vec<String>,
		applications: Vec<SampleDataApplication>,
		super_user: String,
		users: HashMap<String, Vec<String>>,
	}

	#[derive(Debug, Deserialize)]
	#[serde(rename_all = "camelCase")]
	struct SampleData {
		users: Vec<SampleDataUser>,
		organisations: Vec<SampleDataOrganisation>,
	}

	fn get_user_by_username<'a>(
		users: &'a [SampleDataUser],
		username: &str,
	) -> &'a SampleDataUser {
		users.iter().find(|user| user.username == username).unwrap()
	}

	// Wait for a second before starting
	log::warn!("Giving server time to initalize before populating data");
	tokio::time::sleep(Duration::from_secs(2)).await;
	log::info!("Populating database with sample data...");

	let content = include_str!("../../../assets/sample-data.json");
	let data: SampleData = serde_json::from_str(content).unwrap();

	// Create all users
	for user in &data.users {
		let response: Value = Request::new(
			Method::POST,
			Url::parse(
				format!("http://localhost:{}/auth/sign-up", config.config.port)
					.as_ref(),
			)
			.unwrap(),
		)
		.body_json(&json!({
			"username": user.username,
			"email": user.backup_email,
			"password": user.password,
			"accountType": "personal",
			"firstName": user.first_name,
			"lastName": user.last_name
		}))
		.unwrap()
		.await
		.unwrap()
		.body_json()
		.await
		.unwrap();

		if !response["success"].as_bool().unwrap() {
			log::error!(
				"Error signing up user `{}`: {}",
				user.username,
				response
			);
			continue;
		}

		let response: Value = Request::new(
			Method::POST,
			Url::parse(
				format!("http://localhost:{}/auth/join", config.config.port)
					.as_ref(),
			)
			.unwrap(),
		)
		.body_json(&json!({
			"username": user.username,
			"verificationToken": "000-000"
		}))
		.unwrap()
		.await
		.unwrap()
		.body_json()
		.await
		.unwrap();

		if !response["success"].as_bool().unwrap() {
			log::error!("Error joining user {}: {}", user.username, response);
		} else {
			log::info!("User `{}` created successfully", user.username);
		}
	}

	for organisation in &data.organisations {
		let super_user =
			get_user_by_username(&data.users, &organisation.super_user);
		let response: Value = Request::new(
			Method::POST,
			Url::parse(
				format!("http://localhost:{}/auth/sign-in", config.config.port)
					.as_ref(),
			)
			.unwrap(),
		)
		.body_json(&json!({
			"userId": super_user.username,
			"password": super_user.password
		}))
		.unwrap()
		.await
		.unwrap()
		.body_json()
		.await
		.unwrap();

		if !response["success"].as_bool().unwrap() {
			log::error!(
				"Error signing in user {}: {}",
				super_user.username,
				response
			);
			continue;
		}
		let token = response[request_keys::ACCESS_TOKEN].as_str().unwrap();

		let response: Value = Request::new(
			Method::POST,
			Url::parse(
				format!("http://localhost:{}/organisation", config.config.port)
					.as_ref(),
			)
			.unwrap(),
		)
		.set_header("Authorization", token)
		.body_json(&json!({
			"domain": organisation.domains.get(0).unwrap(),
			"organisationName": organisation.name
		}))
		.unwrap()
		.await
		.unwrap()
		.body_json()
		.await
		.unwrap();

		if !response["success"].as_bool().unwrap() {
			log::error!(
				"Error creating organisation {}: {}",
				organisation.name,
				response
			);
		} else {
			log::info!(
				"Organisation `{}` created successfully",
				organisation.name
			);
		}
	}
}
