mod ci;

use api_models::{
	models::workspace::infrastructure::deployment::DeploymentStatus,
	utils::{DateTime, Uuid},
};
use chrono::Utc;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use serde_json::json;

use crate::{
	app::{create_eve_app, App},
	db,
	models::{
		error::{id as ErrorId, message as ErrorMessage},
		Action,
		EventData,
	},
	pin_fn,
	service,
	utils::{
		constants::request_keys,
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

	sub_app.use_sub_app("/ci", ci::create_sub_app(app));

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
		// Put the different "content-type" header and body in the queue and log
		// the message there
		service::queue_docker_notification_error(
			&context.get_body()?,
			context.get_content_type().as_str(),
			&request_id,
		)
		.await?;

		return Ok(context);
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
		let (workspace_id_str, image_name) =
			if let Some(value) = repository_name.split_once('/') {
				value
			} else {
				continue;
			};

		let workspace_id = match Uuid::parse_str(workspace_id_str) {
			Ok(workspace_id) => workspace_id,
			Err(err) => {
				log::trace!("request_id: {} - Unable to parse workspace_id: {} - error - {}", request_id, workspace_id_str, err);
				continue;
			}
		};

		log::trace!(
			"request_id: {} - Getting the docker repository info",
			request_id
		);
		let repository = db::get_docker_repository_by_name(
			context.get_database_connection(),
			image_name,
			&workspace_id,
		)
		.await?;
		let repository = if let Some(repository) = repository {
			repository
		} else {
			continue;
		};

		let current_time = DateTime::from(Utc::now());

		log::trace!(
			"request_id: {} - Creating docker repository digest",
			request_id
		);

		let image_size_in_bytes = target
			.references
			.into_iter()
			.filter(|reference| {
				reference.media_type ==
					"application/vnd.docker.image.rootfs.diff.tar.gzip"
			})
			.map(|reference| reference.size)
			.sum();

		if service::docker_repo_storage_limit_crossed(
			context.get_database_connection(),
			&workspace_id,
			image_size_in_bytes as usize,
		)
		.await?
		{
			log::trace!("request_id: {request_id} - Docker repo storage limit is crossed");

			// delete the docker image as it has crossed the limits
			service::queue_delete_docker_registry_image(
				&workspace_id,
				&repository_name,
				&target.digest,
				&target.tag,
				&event.request.addr,
				&config,
				&request_id,
			)
			.await?;

			// now process next event
			continue;
		}

		db::create_docker_repository_digest(
			context.get_database_connection(),
			&repository.id,
			&target.digest,
			image_size_in_bytes,
			&current_time,
		)
		.await?;

		let total_storage =
			db::get_total_size_of_docker_repositories_for_workspace(
				context.get_database_connection(),
				&workspace_id,
			)
			.await?;
		db::update_docker_repo_usage_history(
			context.get_database_connection(),
			&workspace_id,
			&(total_storage as i64),
			&current_time,
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
			&current_time,
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
				&workspace_id,
			)
			.await?;

		log::trace!("request_id: {} - Updating the deployments", request_id);
		for db_deployment in deployments {
			if let DeploymentStatus::Stopped = db_deployment.status {
				continue;
			}

			let (deployment, workspace_id, _, deployment_running_details) =
				service::get_full_deployment_config(
					context.get_database_connection(),
					&db_deployment.id,
					&request_id,
				)
				.await?;

			log::trace!(
				"request_id: {} - Updating deployment_image_digest with deployment id: {} digest and {}",
				request_id,
				deployment.id,
				target.digest
			);
			let repository_id = db_deployment.repository_id.status(500)?;

			if !repository_id.is_nil() {
				db::add_digest_to_deployment_deploy_history(
					context.get_database_connection(),
					&deployment.id,
					&repository_id,
					&target.digest,
					&current_time,
				)
				.await?;

				db::update_current_live_digest_for_deployment(
					context.get_database_connection(),
					&deployment.id,
					&target.digest,
				)
				.await?;
			}

			log::trace!(
				"request_id: {} - Updating the deployment with id: {}",
				request_id,
				deployment.id
			);

			let (image_name, _) =
				service::get_image_name_and_digest_for_deployment_image(
					context.get_database_connection(),
					&deployment.registry,
					&deployment.image_tag,
					&config,
					&request_id,
				)
				.await?;

			db::update_deployment_status(
				context.get_database_connection(),
				&deployment.id,
				&DeploymentStatus::Deploying,
			)
			.await?;

			context.commit_database_transaction().await?;

			service::queue_update_deployment_image(
				&workspace_id,
				&deployment.id,
				&deployment.name,
				&deployment.registry,
				&image_name,
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
