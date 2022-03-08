use api_models::{
	models::workspace::infrastructure::deployment::DeploymentStatus,
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use serde_json::json;
use tokio::task;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{
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
			let full_image_name = format!(
				"{}@{}",
				deployment
					.get_full_image(context.get_database_connection())
					.await?,
				target.digest
			);

			let config = config.clone();

			let request_id = request_id.clone();
			log::trace!(
				"request_id: {} - Updating the deployment with id: {}",
				request_id,
				deployment.id
			);

			task::spawn(async move {
				log::trace!(
					"request_id: {} - Acquiring database connection",
					request_id
				);
				let mut connection = if let Ok(connection) =
					service::get_app().database.acquire().await
				{
					connection
				} else {
					log::error!(
						"request_id: {} - Cannot acquire a db connection",
						request_id
					);
					return;
				};
				log::trace!(
					"request_id: {} - Acquired database connection",
					request_id
				);

				log::trace!(
					"request_id: {} - Pushing image to DOCR",
					request_id
				);
				let result = service::push_to_docr(
					&mut connection,
					&deployment.id,
					&full_image_name,
					&config,
					&request_id,
				)
				.await;

				if let Err(e) = result {
					log::error!(
						"Error pushing image to docr: {}",
						e.get_error()
					);
					return;
				}

				log::trace!(
					"request_id: {} - Pushed image to DOCR",
					request_id
				);

				log::trace!(
					"request_id: {} - Getting full deployment config",
					request_id
				);
				let deployment_config_result =
					service::get_full_deployment_config(
						&mut connection,
						&deployment.id,
						&request_id,
					)
					.await;

				let (deployment, workspace_id, full_image, running_details) =
					if let Ok(deployment_config_result) =
						deployment_config_result
					{
						deployment_config_result
					} else {
						log::error!(
						"request_id: {} - Unable to get full deployment config",
						request_id
					);
						return;
					};

				log::trace!(
					"request_id: {} - Updating the kubernetes deployment",
					request_id
				);
				let update_kubernetes_result =
					service::update_kubernetes_deployment(
						&workspace_id,
						&deployment,
						&full_image,
						&running_details,
						&config,
						&request_id,
					)
					.await;

				if let Err(error) = update_kubernetes_result {
					log::error!(
						"request_id: {} - Error updating k8s deployment: {}",
						request_id,
						error.get_error()
					);
					let _ = db::update_deployment_status(
						&mut connection,
						&deployment.id,
						&DeploymentStatus::Errored,
					)
					.await
					.map_err(|e| {
						log::error!(
							"request_id: {} - Error setting db status: {}",
							request_id,
							e
						);
					});
				}

				let restart_deployment_result = service::restart_deployment(
					&deployment.id,
					&request_id,
					&config,
					&workspace_id,
				)
				.await;

				if let Err(error) = restart_deployment_result {
					log::error!(
						"request_id: {} - Error restarting k8s deployment: {}",
						request_id,
						error.get_error()
					);
					let _ = db::update_deployment_status(
						&mut connection,
						&deployment.id,
						&DeploymentStatus::Errored,
					)
					.await
					.map_err(|e| {
						log::error!(
							"request_id: {} - Error setting db status: {}",
							request_id,
							e
						);
					});
				}
			});
		}
	}

	Ok(context)
}
