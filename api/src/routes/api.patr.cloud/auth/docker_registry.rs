use api_models::utils::Uuid;
use axum::{
	extract::{Query, State},
	headers::{authorization::Basic, Authorization},
	routing::get,
	Json,
	Router,
	TypedHeader,
};
use base64::prelude::*;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
	models::{
		rbac::{permissions, GOD_USER_ID},
		RegistryToken,
		RegistryTokenAccess,
	},
	prelude::*,
	utils::constants::request_keys,
};

/// This function is used to create a router for every endpoint in this file
pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new().route(
		"/docker-registry-token",
		get(docker_registry_token_endpoint)
			.post(|| StatusCode::METHOD_NOT_ALLOWED),
	)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DockerRegistryTokenQuery {
	#[serde(rename_all = "snake_case")]
	Authorize { scope: String },
	#[serde(rename_all = "snake_case")]
	Login {
		client_id: String,
		service: String,
		offline_token: bool,
	},
}

fn docker_registry_error(error_code: &str, message: &str) -> serde_json::Value {
	json!({
		"errors": [{
			"code": error_code,
			"message": message,
			"detail": []
		}]
	})
}

/// This function is used to authorize and login into the docker registry
async fn docker_registry_token_endpoint(
	mut connection: Connection,
	Query(query): Query<DockerRegistryTokenQuery>,
	typed_header: TypedHeader<Authorization<Basic>>,
	state: State<App>,
) -> (StatusCode, Json<serde_json::Value>) {
	match query {
		DockerRegistryTokenQuery::Authorize { scope } => todo!(),
		DockerRegistryTokenQuery::Login {
			client_id,
			service,
			offline_token,
		} => {
			docker_registry_login(
				connection,
				client_id,
				service,
				offline_token,
				typed_header,
				state,
			)
			.await
		}
	}
}

/// This function is used to login into the docker registry
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
async fn docker_registry_login(
	mut connection: Connection,
	_client_id: String,
	service: String,
	_offline_token: bool,
	TypedHeader(authorization): TypedHeader<Authorization<Basic>>,
	State(app): State<App>,
) -> (StatusCode, Json<serde_json::Value>) {
	let config = app.config;

	if service != &config.docker_registry.service_name {
		return (
			StatusCode::BAD_REQUEST,
			Json(docker_registry_error(
				"UNSUPPORTED",
				"Invalid request sent by the client. Service is not valid",
			)),
		);
	}

	let username = authorization.username();
	let password = authorization.password();
	let user = match db::get_user_by_username(&mut connection, username).await {
		Ok(Some(user)) => user,
		Ok(None) => {
			return (
				StatusCode::UNAUTHORIZED,
				Json(docker_registry_error(
					"DENIED",
					"No user found with that username",
				)),
			);
		}
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(docker_registry_error(
					"UNSUPPORTED",
					concat!(
						"An internal server error has occured.",
						" Please try again later"
					),
				)),
			);
		}
	};

	// TODO API token as password instead of password, for TFA.
	let success =
		if let Ok(value) = service::validate_hash(password, &user.password) {
			value
		} else {
			return (
				StatusCode::BAD_REQUEST,
				Json(docker_registry_error(
					"UNSUPPORTED",
					concat!(
						"An internal server error has occured.",
						" Please try again later"
					),
				)),
			);
		};

	if !success {
		return (
			StatusCode::UNAUTHORIZED,
			Json(docker_registry_error(
				"DENIED",
				"Your password is incorrect",
			)),
		);
	}

	RegistryToken::new(
		config.docker_registry.issuer.clone(),
		Utc::now(),
		username.to_string(),
		&config,
		vec![],
	)
	.to_string(
		config.docker_registry.private_key.as_ref(),
		config.docker_registry.public_key_der.as_ref(),
	)
	.map(|token| (StatusCode::OK, Json(json!({ request_keys::TOKEN: token }))))
	.unwrap_or_else(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(docker_registry_error(
				"UNSUPPORTED",
				concat!(
					"An internal server error has occured.",
					" Please try again later"
				),
			)),
		)
	})
}

/// This function is used to authorize a user's permissions for docker registry
async fn docker_registry_authorize(
	mut connection: Connection,
	scope: String,
	TypedHeader(authorization): TypedHeader<Authorization<Basic>>,
	State(app): State<App>,
) -> (StatusCode, Json<serde_json::Value>) {
	let config = app.config;

	let username = authorization.username();
	let password = authorization.password();
	let user = match db::get_user_by_username(&mut connection, username).await {
		Ok(Some(user)) => user,
		Ok(None) => {
			return (
				StatusCode::UNAUTHORIZED,
				Json(docker_registry_error(
					"DENIED",
					"No user found with that username",
				)),
			);
		}
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(docker_registry_error(
					"UNSUPPORTED",
					concat!(
						"An internal server error has occured.",
						" Please try again later"
					),
				)),
			);
		}
	};

	// TODO API token for password instead of password, for TFA.
	let success =
		if let Ok(value) = service::validate_hash(password, &user.password) {
			value
		} else {
			return (
				StatusCode::BAD_REQUEST,
				Json(docker_registry_error(
					"UNSUPPORTED",
					concat!(
						"An internal server error has occured.",
						" Please try again later"
					),
				)),
			);
		};

	if !success {
		return (
			StatusCode::UNAUTHORIZED,
			Json(docker_registry_error(
				"DENIED",
				"Your password is incorrect",
			)),
		);
	}

	let (access_type, remaining) = if let Some(value) = scope.split_once(':') {
		value
	} else {
		return (
			StatusCode::BAD_REQUEST,
			Json(docker_registry_error(
				"UNSUPPORTED",
				"Access type not present in request",
			)),
		);
	};
	let (repo, action) = if let Some(value) = remaining.split_once(':') {
		value
	} else {
		return (
			StatusCode::BAD_REQUEST,
			Json(docker_registry_error(
				"UNSUPPORTED",
				"Action not present in request",
			)),
		);
	};

	// check if access type is repository
	if access_type != "repository" {
		return (
			StatusCode::BAD_REQUEST,
			Json(docker_registry_error(
				"UNSUPPORTED",
				"Invalid access type sent by client",
			)),
		);
	}

	let required_permissions = action
		.split(',')
		.filter_map(|permission| match permission {
			"push" | "tag" => {
				Some(permissions::workspace::docker_registry::PUSH)
			}
			"pull" => Some(permissions::workspace::docker_registry::PULL),
			_ => None,
		})
		.map(String::from)
		.collect::<Vec<_>>();

	let (workspace_id_str, repo_name) =
		if let Some(value) = repo.split_once('/') {
			value
		} else {
			return (
				StatusCode::BAD_REQUEST,
				Json(docker_registry_error(
					"NAME_INVALID",
					"Invalid workspace or repository name",
				)),
			);
		};

	let workspace_id = if let Ok(uuid) = Uuid::parse_str(workspace_id_str) {
		uuid
	} else {
		return (
			StatusCode::BAD_REQUEST,
			Json(docker_registry_error(
				"NAME_INVALID",
				"Invalid workspace ID",
			)),
		);
	};

	// check if repo name is valid
	let is_repo_name_valid = validator::is_docker_repo_name_valid(repo_name);
	if !is_repo_name_valid {
		return (
			StatusCode::BAD_REQUEST,
			Json(docker_registry_error(
				"NAME_INVALID",
				"Invalid repository name",
			)),
		);
	}
	match db::get_workspace_info(&mut connection, &workspace_id).await {
		Ok(Some(_)) => (),
		Ok(None) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(docker_registry_error(
					"NAME_UNKNOWN",
					"Workspace does not exist",
				)),
			);
		}
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(docker_registry_error(
					"UNSUPPORTED",
					concat!(
						"An internal server error has occured.",
						" Please try again later"
					),
				)),
			);
		}
	};

	let repository = match db::get_docker_repository_by_name(
		&mut connection,
		repo_name,
		&workspace_id,
	)
	.await
	{
		Ok(Some(repository)) => repository,
		Ok(None) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(docker_registry_error(
					"NAME_UNKNOWN",
					"Repository does not exist",
				)),
			);
		}
		Err(_) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(docker_registry_error(
					"UNSUPPORTED",
					concat!(
						"An internal server error has occured.",
						" Please try again later"
					),
				)),
			);
		}
	};

	// get repo id inorder to get resource details
	let resource = if let Ok(Some(resource)) =
		db::get_resource_by_id(&mut connection, &repository.id).await
	{
		resource
	} else {
		return (
			StatusCode::BAD_REQUEST,
			Json(docker_registry_error(
				"UNSUPPORTED",
				concat!(
					"An internal server error has occured.",
					" Please try again later"
				),
			)),
		);
	};

	if resource.owner_id != workspace_id {
		log::error!(
			"Resource owner_id is not the same as workspace id. This is illegal"
		);
		return (
			StatusCode::BAD_REQUEST,
			Json(docker_registry_error(
				"UNSUPPORTED",
				concat!(
					"An internal server error has occured.",
					" Please try again later"
				),
			)),
		);
	}

	let god_user_id = GOD_USER_ID.get().unwrap();

	// get all workspace roles for the user using the id
	let user_id = &user.id;
	let user_roles = if let Ok(user_roles) =
		db::get_all_workspace_role_permissions_for_user(
			&mut connection,
			&user.id,
		)
		.await
	{
		user_roles
	} else {
		return (
			StatusCode::BAD_REQUEST,
			Json(docker_registry_error(
				"UNSUPPORTED",
				concat!(
					"An internal server error has occured.",
					" Please try again later"
				),
			)),
		);
	};

	let required_role_for_user = user_roles.get(&workspace_id);
	let mut approved_permissions = vec![];

	for permission in required_permissions {
		let allowed =
			if let Some(required_role_for_user) = required_role_for_user {
				let resource_type_allowed = {
					if let Some(permissions) = required_role_for_user
						.resource_type_permissions
						.get(&resource.resource_type_id)
					{
						permissions.contains(
							rbac::PERMISSIONS
								.get()
								.unwrap()
								.get(&(*permission).to_string())
								.unwrap(),
						)
					} else {
						false
					}
				};
				let resource_allowed = {
					if let Some(permissions) = required_role_for_user
						.resource_permissions
						.get(&resource.id)
					{
						permissions.contains(
							rbac::PERMISSIONS
								.get()
								.unwrap()
								.get(&(*permission).to_string())
								.unwrap(),
						)
					} else {
						false
					}
				};
				let is_super_admin = {
					required_role_for_user.is_super_admin || {
						user_id == god_user_id
					}
				};
				resource_type_allowed || resource_allowed || is_super_admin
			} else {
				user_id == god_user_id
			};
		if !allowed {
			continue;
		}

		match permission.as_str() {
			permissions::workspace::docker_registry::PUSH => {
				approved_permissions.push("push".to_string());
			}
			permissions::workspace::docker_registry::PULL => {
				approved_permissions.push("pull".to_string());
			}
			_ => {}
		}
	}

	RegistryToken::new(
		config.docker_registry.issuer.clone(),
		Utc::now(),
		username.to_string(),
		&config,
		vec![RegistryTokenAccess {
			r#type: access_type.to_string(),
			name: repo.to_string(),
			actions: approved_permissions,
		}],
	)
	.to_string(
		config.docker_registry.private_key.as_ref(),
		config.docker_registry.public_key_der.as_ref(),
	)
	.map(|token| (StatusCode::OK, Json(json!({ request_keys::TOKEN: token }))))
	.unwrap_or_else(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(docker_registry_error(
				"UNSUPPORTED",
				concat!(
					"An internal server error has occured.",
					" Please try again later"
				),
			)),
		)
	})
}
