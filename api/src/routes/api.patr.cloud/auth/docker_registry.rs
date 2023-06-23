use std::net::{IpAddr, Ipv4Addr};

use api_models::utils::Uuid;
use base64::prelude::*;
use chrono::Utc;
use eve_rs::{App as EveApp, AsError, Context, HttpMethod, NextHandler};
use serde_json::json;

use crate::{
	app::{create_eve_app, App},
	db::{self},
	models::{
		error::{id as ErrorId, message as ErrorMessage},
		is_user_action_authorized,
		rbac::{self, permissions},
		ApiTokenData,
		RegistryToken,
		RegistryTokenAccess,
	},
	pin_fn,
	routes::get_request_ip_address,
	service,
	utils::{
		constants::request_keys,
		validator,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};

/// # Description
/// This function is used to create a sub app for every endpoint listed. It
/// creates an eve app which binds the endpoint with functions.
///
/// # Arguments
/// * `app` - an object of type [`App`] which contains all the configuration of
/// api including the database connections.
///
/// # Returns
/// this function returns `EveApp<EveContext, EveMiddleware, App, ErrorData>`
/// containing context, middleware, object of [`App`] and Error
///
/// [`App`]: App
pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut app = create_eve_app(app);

	app.post(
		"/docker-registry-token",
		[EveMiddleware::CustomFunction(pin_fn!(
			docker_registry_token_endpoint
		))],
	);
	app.get(
		"/docker-registry-token",
		[EveMiddleware::CustomFunction(pin_fn!(
			docker_registry_token_endpoint
		))],
	);

	app
}

/// # Description
/// This function is used to authenticate and login into the docker registry
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// ```
/// {
///    scope:
///    client_id:
///    service:
///    offline_token:
/// }
/// ```
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    token:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn docker_registry_token_endpoint(
	mut context: EveContext,
	next: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let query = context.get_request().get_query();

	if context.get_method() == &HttpMethod::Post {
		context.status(405);
		return Ok(context);
	}

	if query.get(request_keys::SCOPE).is_some() {
		// Authenticating an existing login
		docker_registry_authenticate(context, next).await
	} else {
		// Logging in
		docker_registry_login(context, next).await
	}
}

/// # Description
/// This function is used to login into the docker registry
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// ```
/// {
///    client_id: ,
///    offline_token: ,
///    service:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    token:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn docker_registry_login(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let query = context.get_request().get_query().clone();
	let config = context.get_state().config.clone();

	let _client_id = query
		.get(request_keys::SNAKE_CASE_CLIENT_ID)
		.status(400)
		.body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::UNAUTHORIZED,
					request_keys::MESSAGE: ErrorMessage::INVALID_CLIENT_ID,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;

	let _offline_token = query
		.get(request_keys::SNAKE_CASE_OFFLINE_TOKEN)
		.map(|value| {
			value.parse::<bool>().status(400).body(
				json!({
					request_keys::ERRORS: [{
						request_keys::CODE: ErrorId::UNAUTHORIZED,
						request_keys::MESSAGE: ErrorMessage::INVALID_OFFLINE_TOKEN,
						request_keys::DETAIL: []
					}]
				})
				.to_string(),
			)
		})
		.status(400)
		.body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::UNAUTHORIZED,
					request_keys::MESSAGE: ErrorMessage::OFFLINE_TOKEN_NOT_FOUND,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)??;

	let service = query.get(request_keys::SERVICE).status(400).body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::UNAUTHORIZED,
				request_keys::MESSAGE: ErrorMessage::SERVICE_NOT_FOUND,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;

	if service != &config.docker_registry.service_name {
		Error::as_result().status(400).body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::UNAUTHORIZED,
					request_keys::MESSAGE: ErrorMessage::INVALID_SERVICE,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;
	}

	let authorization = context
		.get_header("Authorization")
		.map(|value| value.replace("Basic ", ""))
		.map(|value| {
			BASE64_STANDARD
				.decode(value)
				.ok()
				.and_then(|value| String::from_utf8(value).ok())
				.status(400)
				.body(
					json!({
						request_keys::ERRORS: [{
							request_keys::CODE: ErrorId::UNAUTHORIZED,
							request_keys::MESSAGE: ErrorMessage::AUTHORIZATION_PARSE_ERROR,
							request_keys::DETAIL: []
						}]
					})
					.to_string(),
				)
		})
		.status(400)
		.body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::UNAUTHORIZED,
					request_keys::MESSAGE: ErrorMessage::AUTHORIZATION_NOT_FOUND,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)??;

	let mut splitter = authorization.split(':');
	let username = splitter.next().status(400).body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::UNAUTHORIZED,
				request_keys::MESSAGE: ErrorMessage::USERNAME_NOT_FOUND,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;
	let password = splitter.next().status(400).body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::UNAUTHORIZED,
				request_keys::MESSAGE: ErrorMessage::PASSWORD_NOT_FOUND,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;
	let user =
		db::get_user_by_username(context.get_database_connection(), username)
			.await?
			.status(401)
			.body(
				json!({
					request_keys::ERRORS: [{
						request_keys::CODE: ErrorId::UNAUTHORIZED,
						request_keys::MESSAGE: ErrorMessage::USER_NOT_FOUND,
						request_keys::DETAIL: []
					}]
				})
				.to_string(),
			)?;

	let auth_success = 'auth: {
		if password.starts_with("patrv1") {
			// the user might have used api_token as password,
			// so first check whether that api token has valid permission or not
			let mut redis_connection = context.get_redis_connection().clone();
			let accessing_ip = get_request_ip_address(&context)
				.parse()
				.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
			let api_token = ApiTokenData::decode(
				context.get_database_connection(),
				&mut redis_connection,
				password,
				&accessing_ip,
			)
			.await;

			match api_token {
				Ok(token) => {
					// since username is used in sub, check whether provided
					// username is valid with token's username
					break 'auth db::get_user_by_user_id(
						context.get_database_connection(),
						&token.user_id,
					)
					.await?
					.map_or(false, |api_user| api_user.username == username);
				}
				Err(err) => {
					log::error!("Error while decoding api token: {err:?}");
				}
			}
		}

		// the user might have password which might start 'patrv1'
		// so check for normal username password authentication too
		service::validate_hash(password, &user.password)?
	};

	if !auth_success {
		Error::as_result().status(401).body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::UNAUTHORIZED,
					request_keys::MESSAGE: ErrorMessage::INVALID_PASSWORD,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;
	}

	let token = RegistryToken::new(
		config.docker_registry.issuer.clone(),
		Utc::now(),
		username.to_string(),
		&config,
		vec![],
	)
	.to_string(
		config.docker_registry.private_key.as_ref(),
		config.docker_registry.public_key_der.as_ref(),
	)?;

	context.json(json!({ request_keys::TOKEN: token }));
	Ok(context)
}

/// # Description
/// This function is used to authenticate the user for docker registry
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    token:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn docker_registry_authenticate(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let query = context.get_request().get_query().clone();
	let config = context.get_state().config.clone();

	let authorization = context
		.get_header("Authorization")
		.map(|value| value.replace("Basic ", ""))
		.map(|value| {
			BASE64_STANDARD
				.decode(value)
				.ok()
				.and_then(|value| String::from_utf8(value).ok())
				.status(400)
				.body(
					json!({
						request_keys::ERRORS: [{
							request_keys::CODE: ErrorId::UNAUTHORIZED,
							request_keys::MESSAGE: ErrorMessage::AUTHORIZATION_PARSE_ERROR,
							request_keys::DETAIL: []
						}]
					})
					.to_string(),
				)
		})
		.status(400)
		.body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::UNAUTHORIZED,
					request_keys::MESSAGE: ErrorMessage::AUTHORIZATION_NOT_FOUND,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)??;

	let mut splitter = authorization.split(':');
	let username = splitter.next().status(400).body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::UNAUTHORIZED,
				request_keys::MESSAGE: ErrorMessage::USERNAME_NOT_FOUND,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;
	let password = splitter.next().status(400).body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::UNAUTHORIZED,
				request_keys::MESSAGE: ErrorMessage::PASSWORD_NOT_FOUND,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;
	let user =
		db::get_user_by_username(context.get_database_connection(), username)
			.await?
			.status(401)
			.body(
				json!({
					request_keys::ERRORS: [{
						request_keys::CODE: ErrorId::UNAUTHORIZED,
						request_keys::MESSAGE: ErrorMessage::USER_NOT_FOUND,
						request_keys::DETAIL: []
					}]
				})
				.to_string(),
			)?;

	let god_user_id = rbac::GOD_USER_ID.get().unwrap();
	// check if user is GOD_USER then return the token
	if &user.id == god_user_id {
		// return token.
		if RegistryToken::parse(
			password,
			context
				.get_state()
				.config
				.docker_registry
				.public_key
				.as_bytes(),
		)
		.is_ok()
		{
			context.json(json!({ request_keys::TOKEN: password }));
			return Ok(context);
		}
	}

	let auth_success = 'auth: {
		if password.starts_with("patrv1") {
			// the user might have used api_token as password,
			// so first check whether that api token has valid permission or not
			let mut redis_connection = context.get_redis_connection().clone();
			let accessing_ip = get_request_ip_address(&context)
				.parse()
				.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
			let api_token = ApiTokenData::decode(
				context.get_database_connection(),
				&mut redis_connection,
				password,
				&accessing_ip,
			)
			.await;

			match api_token {
				Ok(token) => {
					// since username is used in sub, check whether provided
					// username is valid with token's username
					break 'auth db::get_user_by_user_id(
						context.get_database_connection(),
						&token.user_id,
					)
					.await?
					.and_then(|api_user| {
						if api_user.username == username {
							Some(token.permissions)
						} else {
							None
						}
					});
				}
				Err(err) => {
					log::error!("Error while decoding api token: {err:?}");
				}
			}
		}

		// the user might have password which might start 'patrv1'
		// so check for normal username password authentication too
		if service::validate_hash(password, &user.password)? {
			let user_roles = db::get_all_workspace_role_permissions_for_user(
				context.get_database_connection(),
				&user.id,
			)
			.await?;
			Some(user_roles)
		} else {
			None
		}
	};

	let Some(user_permissions) = auth_success else {
		return Error::as_result().status(401).body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::UNAUTHORIZED,
					request_keys::MESSAGE: ErrorMessage::INVALID_PASSWORD,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;
	};

	let scope = query.get(request_keys::SCOPE).status(500)?;
	let mut splitter = scope.split(':');
	let access_type = splitter.next().status(401).body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::INVALID_REQUEST,
				request_keys::MESSAGE: ErrorMessage::ACCESS_TYPE_NOT_PRESENT,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;

	// check if access type is repository
	if access_type != request_keys::REPOSITORY {
		Error::as_result().status(400).body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::INVALID_REQUEST,
					request_keys::MESSAGE: ErrorMessage::INVALID_ACCESS_TYPE,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;
	}

	let repo = splitter.next().status(400).body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::INVALID_REQUEST,
				request_keys::MESSAGE: ErrorMessage::REPOSITORY_NOT_PRESENT,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;

	let action = splitter.next().status(400).body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::INVALID_REQUEST,
				request_keys::MESSAGE: ErrorMessage::ACTION_NOT_PRESENT,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;

	let required_permissions = action
		.split(',')
		.filter_map(|permission| match permission {
			"push" | "tag" => {
				Some(permissions::workspace::container_registry::PUSH)
			}
			"pull" => Some(permissions::workspace::container_registry::PULL),
			_ => None,
		})
		.map(String::from)
		.collect::<Vec<_>>();

	let split_array = repo.split('/').map(String::from).collect::<Vec<_>>();
	// reject if split array size is not equal to 2
	if split_array.len() != 2 {
		Error::as_result().status(400).body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::INVALID_REQUEST,
					request_keys::MESSAGE: ErrorMessage::NO_WORKSPACE_OR_REPOSITORY,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;
	}

	let workspace_id_str = split_array.get(0).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id_str)
		.map_err(|err| {
			log::trace!(
				"Unable to parse workspace_id: {} - error - {}",
				workspace_id_str,
				err
			);
			err
		})
		.status(500)
		.body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::INVALID_REQUEST,
					request_keys::MESSAGE: ErrorMessage::NO_WORKSPACE_OR_REPOSITORY,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;

	// get first index from the vector
	let repo_name = split_array.get(1).unwrap();

	// check if repo name is valid
	let is_repo_name_valid = validator::is_docker_repo_name_valid(repo_name);
	if !is_repo_name_valid {
		Error::as_result().status(400).body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::INVALID_REQUEST,
					request_keys::MESSAGE: ErrorMessage::INVALID_REPOSITORY_NAME,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;
	}
	db::get_workspace_info(context.get_database_connection(), &workspace_id)
		.await?
		.status(400)
		.body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::INVALID_REQUEST,
					request_keys::MESSAGE: ErrorMessage::RESOURCE_DOES_NOT_EXIST,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;

	let repository = db::get_docker_repository_by_name(
		context.get_database_connection(),
		repo_name,
		&workspace_id,
	)
	.await?
	// reject request if repository does not exist
	.status(400)
	.body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::INVALID_REQUEST,
				request_keys::MESSAGE: ErrorMessage::RESOURCE_DOES_NOT_EXIST,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;

	// get repo id inorder to get resource details
	let resource = db::get_resource_by_id(
		context.get_database_connection(),
		&repository.id,
	)
	.await?
	.status(500)
	.body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::SERVER_ERROR,
				request_keys::MESSAGE: ErrorMessage::SERVER_ERROR,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;

	if resource.owner_id != workspace_id {
		log::error!(
			"Resource owner_id is not the same as workspace id. This is illegal"
		);
		Error::as_result().status(500).body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::SERVER_ERROR,
					request_keys::MESSAGE: ErrorMessage::SERVER_ERROR,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;
	}

	// get all workspace roles for the user using the id
	let mut approved_permissions = vec![];

	for permission in required_permissions {
		let allowed = is_user_action_authorized(
			&user_permissions,
			&user.id,
			&workspace_id,
			&permission,
			&resource.id,
		);
		if !allowed {
			continue;
		}

		match permission.as_str() {
			permissions::workspace::container_registry::PUSH => {
				approved_permissions.push("push".to_string());
			}
			permissions::workspace::container_registry::PULL => {
				approved_permissions.push("pull".to_string());
			}
			_ => {}
		}
	}

	let token = RegistryToken::new(
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
	)?;

	context.json(json!({ request_keys::TOKEN: token }));
	Ok(context)
}
