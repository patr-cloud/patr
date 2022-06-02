use api_models::{
	models::workspace::infrastructure::deployment::DeploymentStatus,
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use serde_json::json;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{
		deployment::KubernetesEventData,
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

	sub_app.post(
		"/kubernetes-events",
		[EveMiddleware::CustomFunction(pin_fn!(deployment_alert))],
	);

	sub_app
}

/// # Description
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
async fn notification_handler(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {} - Received notification from docker registry",
		request_id,
	);

	log::trace!("request_id: {} - Checking the content type", request_id);
	if context.get_content_type().as_str() !=
		"application/vnd.docker.distribution.events.v1+json"
	{
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	log::trace!(
		"request_id: {} - Checking the Authorization header",
		request_id
	);
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
	log::trace!(
		"request_id: {} - Parsing the Custom Authorization header",
		request_id
	);
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
	for event in events.events.into_iter().filter(|event| {
		// Only process events that are push events of a manifest
		(event.action == Action::Push || event.action == Action::Mount) &&
			(event.target.media_type ==
				"application/vnd.docker.distribution.manifest.v2+json")
	}) {
		let target = event.target;

		// Update the docker registry db with details on the image
		let repository_name = target.repository;
		let (workspace_name, image_name) =
			if let Some(value) = repository_name.split_once('/') {
				value
			} else {
				continue;
			};

		log::trace!("request_id: {} - Getting the workspace", request_id);
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

		log::trace!(
			"request_id: {} - Getting the docker repository info",
			request_id
		);
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

		log::trace!(
			"request_id: {} - Creating docker repository digest",
			request_id
		);
		db::create_docker_repository_digest(
			context.get_database_connection(),
			&repository.id,
			&target.digest,
			target
				.references
				.into_iter()
				.filter(|reference| {
					reference.media_type ==
						"application/vnd.docker.image.rootfs.diff.tar.gzip"
				})
				.map(|reference| reference.size)
				.sum(),
			current_time,
		)
		.await?;

		if target.tag.is_empty() {
			continue;
		}

		log::trace!(
			"request_id: {} - Setting the docker repositorty tag details",
			request_id
		);
		db::set_docker_repository_tag_details(
			context.get_database_connection(),
			&repository.id,
			&target.tag,
			&target.digest,
			current_time,
		)
		.await?;

		log::trace!(
			"request_id: {} - Getting the deployments by image name and tag",
			request_id
		);
		let deployments =
			db::get_deployments_by_image_name_and_tag_for_workspace(
				context.get_database_connection(),
				image_name,
				&target.tag,
				&workspace.id,
			)
			.await?;

		log::trace!("request_id: {} - Updating the deployments", request_id);
		for deployment in deployments {
			if let DeploymentStatus::Stopped = deployment.status {
				continue;
			}

			let (deployment, workspace_id, _, deployment_running_details) =
				service::get_full_deployment_config(
					context.get_database_connection(),
					&deployment.id,
					&request_id,
				)
				.await?;

			log::trace!(
				"request_id: {} - Updating the deployment with id: {}",
				request_id,
				deployment.id
			);

			service::queue_update_deployment_image(
				context.get_database_connection(),
				&workspace_id,
				&deployment.id,
				&deployment.name,
				&deployment.registry,
				&target.digest,
				&deployment.image_tag,
				&deployment.region,
				&deployment.machine_type,
				&deployment_running_details,
				&config,
				&request_id,
			)
			.await?;
		}
	}

	Ok(context)
}

/// # Description
/// This function is used to catch the alerts coming from the kubernetes
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
async fn deployment_alert(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {} - Checking the content type of the request",
		request_id
	);

	if context.get_content_type().as_str() != "application/json" {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	}

	log::trace!(
		"request_id: {} - Checking the Authorization header",
		request_id
	);
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
	log::trace!(
		"request_id: {} - Parsing the Custom Authorization header",
		request_id
	);

	if custom_header != config.kubernetes.alert_webhook_secret {
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

	log::trace!("request_id: {} - Parsing the kubernetes events", request_id);
	let kube_events: KubernetesEventData = serde_json::from_str(&body)?;

	match kube_events.message {
		message
			if message.contains("Back-off restarting failed container") ||
				message.contains("CrashLoopBackOff") =>
		{
			log::trace!(
				"request_id: {} - getting deployment and user info",
				request_id
			);
			let workspace_id =
				Uuid::parse_str(&kube_events.involved_object.namespace)?;

			let workspace = db::get_workspace_info(
				context.get_database_connection(),
				&workspace_id,
			)
			.await?
			.status(500)?;

			let deployment_id = kube_events
				.involved_object
				.name
				.split('-')
				.collect::<Vec<&str>>()
				.into_iter()
				.nth(1)
				.status(500)?;

			let deployment_id = Uuid::parse_str(deployment_id)?;

			let deployment = db::get_deployment_by_id(
				context.get_database_connection(),
				&deployment_id,
			)
			.await?
			.status(500)?;

			log::trace!("request_id: {} - Sending the alert to the user's registered email address", request_id);
			service::send_alert_email(
				&workspace.name,
				&deployment_id,
				&deployment.name,
				"The deployment encountered some errror please check logs to find out.",
				workspace.alert_emails
			)
			.await?;
		}
		message
			if message.contains("Back-off pulling image") ||
				message.contains("Failed to pull") ||
				message.contains("ImagePullBackOff") =>
		{
			log::trace!(
				"request_id: {} - getting deployment and user info",
				request_id
			);
			let workspace_id =
				Uuid::parse_str(&kube_events.involved_object.namespace)?;

			let workspace = db::get_workspace_info(
				context.get_database_connection(),
				&workspace_id,
			)
			.await?
			.status(500)?;

			let deployment_id = kube_events
				.involved_object
				.name
				.split('-')
				.collect::<Vec<&str>>()
				.into_iter()
				.nth(1)
				.status(500)?;

			let deployment_id = Uuid::parse_str(deployment_id)?;

			let deployment = db::get_deployment_by_id(
				context.get_database_connection(),
				&deployment_id,
			)
			.await?
			.status(500)?;

			let error_message = r#"
					There was an issue regading your image. 
					We are unable to pull your image. 
					Please re-check your image url, repository and tag in the deployment and push the image again
				"#;

			log::trace!("request_id: {} - Sending the alert to the user's registered email address", request_id);
			service::send_alert_email(
				&workspace.name,
				&deployment_id,
				&deployment.name,
				error_message,
				workspace.alert_emails,
			)
			.await?;
		}
		_ => {
			log::trace!(
				"request_id: {} - Sending the alert to the patr alert email",
				request_id
			);
			service::send_alert_email_to_patr(kube_events).await?;
		}
	}

	Ok(context)
}
