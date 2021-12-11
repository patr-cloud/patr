use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use serde_json::json;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{
		db_mapping::DeploymentStatus,
		error::{id as ErrorId, message as ErrorMessage},
		Action,
		EventData,
	},
	pin_fn,
	service,
	utils::{
		constants::request_keys,
		get_current_time_millis,
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
///   api including the
/// database connections.
///
/// # Returns
/// this function returns `EveApp<EveContext, EveMiddleware, App, ErrorData>`
/// containing context, middleware, object of [`App`] and Error
///
/// [`App`]: App
pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut sub_app = create_eve_app(app);

	sub_app.post(
		"/docker-registry/notification",
		[EveMiddleware::CustomFunction(pin_fn!(notification_handler))],
	);

	// add logs for requests made to deployment
	sub_app.post(
		"/deployment-request-log",
		[EveMiddleware::CustomFunction(pin_fn!(
			add_deployment_request_log
		))],
	);

	sub_app
}

/// # Description
/// This function is used to handle all the notifications of the API.
/// This function will detect a push being made to a tag, and in case a
/// deployment exists with the given tag, it will automatically update the
/// `deployed_image` of the given [`Deployment`] in the database
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
/// [`EveContext`] or an error
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
/// [`Deployment`]: Deployment
pub async fn notification_handler(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	if context.get_content_type().as_str() !=
		"application/vnd.docker.distribution.events.v1+json"
	{
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	let custom_header = context.get_header("Authorization").status(400).body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::UNAUTHORIZED,
				request_keys::MESSAGE: ErrorMessage::AUTHORIZATION_NOT_FOUND,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;

	let config = context.get_state().config.clone();
	if custom_header != config.docker_registry.authorization_header {
		Error::as_result().status(400).body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::UNAUTHORIZED,
					request_keys::MESSAGE: ErrorMessage::AUTHORIZATION_PARSE_ERROR,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;
	}

	let body = context.get_body()?;
	let events: EventData = serde_json::from_str(&body)?;

	// check if the event is a push event
	// get image name, repository name, tag if present
	for event in events.events {
		if event.action != Action::Push && event.action != Action::Mount {
			continue;
		}
		let target = event.target;

		// Update the docker registry db with details on the image
		let repository_name = target.repository;
		let mut splitter = repository_name.split('/');
		let workspace_name = if let Some(val) = splitter.next() {
			val
		} else {
			continue;
		};
		let image_name = if let Some(val) = splitter.next() {
			val
		} else {
			continue;
		};

		let workspace = db::get_workspace_by_name(
			context.get_database_connection(),
			workspace_name,
		)
		.await?;
		let workspace = if let Some(workspace) = workspace {
			workspace
		} else {
			continue;
		};

		let repository = db::get_docker_repository_by_name(
			context.get_database_connection(),
			image_name,
			&workspace.id,
		)
		.await?;
		let repository = if let Some(repository) = repository {
			repository
		} else {
			continue;
		};

		let current_time = get_current_time_millis();

		db::create_docker_repository_digest(
			context.get_database_connection(),
			&repository.id,
			&target.digest,
			target.size,
			current_time,
		)
		.await?;

		if target.tag.is_empty() {
			continue;
		}

		db::set_docker_repository_tag_details(
			context.get_database_connection(),
			&repository.id,
			&target.tag,
			&target.digest,
			current_time,
		)
		.await?;

		let deployments =
			db::get_deployments_by_image_name_and_tag_for_workspace(
				context.get_database_connection(),
				image_name,
				&target.tag,
				&workspace.id,
			)
			.await?;

		for deployment in deployments {
			if let DeploymentStatus::Stopped = deployment.status {
				continue;
			}
			let full_image_name = format!(
				"{}@{}",
				deployment
					.get_full_image(context.get_database_connection())
					.await?,
				target.digest
			);

			db::update_deployment_deployed_image(
				context.get_database_connection(),
				&deployment.id,
				Some(&full_image_name),
			)
			.await?;

			service::start_deployment(
				context.get_database_connection(),
				&deployment.id,
				&config,
			)
			.await?;
		}
	}

	Ok(context)
}

/// # Description
/// This function is used to log the requests made for the deployment
/// If a user makes a request to a deployment this function will log it and
/// store it in the database
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
/// [`EveContext`] or an error
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
/// [`Deployment`]: Deployment
async fn add_deployment_request_log(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let custom_header = context.get_header("Authorization").status(400).body(
		json!({
			request_keys::ERRORS: [{
				request_keys::CODE: ErrorId::UNAUTHORIZED,
				request_keys::MESSAGE: ErrorMessage::AUTHORIZATION_NOT_FOUND,
				request_keys::DETAIL: []
			}]
		})
		.to_string(),
	)?;

	let config = context.get_state().config.clone();
	if custom_header != config.docker_registry.registry_url {
		Error::as_result().status(400).body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::UNAUTHORIZED,
					request_keys::MESSAGE: ErrorMessage::AUTHORIZATION_PARSE_ERROR,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;
	}
	let body = context.get_body_object().clone();

	let ip_address = body
		.get(request_keys::IP_ADDRESS)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let method = body
		.get(request_keys::METHOD)
		.map(|value| value.as_str())
		.flatten()
		.map(|method| method.parse().ok())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let host = body
		.get(request_keys::DOMAIN)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let protocol = body
		.get(request_keys::PROTOCOL)
		.map(|value| value.as_str())
		.flatten()
		.map(|protocol| protocol.parse().ok())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let path = body
		.get(request_keys::PATH)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let response_time = body
		.get(request_keys::RESPONSE_TIME)
		.map(|value| value.as_f64())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let deployment_id = if host.ends_with(".patr.cloud") {
		let deployment_id_string = host.replace(".patr.cloud", "");
		let deployment_id = if let Ok(id) = hex::decode(deployment_id_string) {
			id
		} else {
			return Ok(context);
		};
		if db::get_deployment_by_id(
			context.get_database_connection(),
			&deployment_id,
		)
		.await?
		.is_none()
		{
			return Ok(context);
		}
		deployment_id
	} else {
		// get deployment by domain_name
		if let Some(deployment) = db::get_deployment_by_domain_name(
			context.get_database_connection(),
			host,
		)
		.await?
		{
			deployment.id
		} else {
			return Ok(context);
		}
	};

	service::create_request_log_for_deployment(
		context.get_database_connection(),
		&deployment_id,
		get_current_time_millis(),
		ip_address,
		&method,
		host,
		&protocol,
		path,
		response_time,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}
