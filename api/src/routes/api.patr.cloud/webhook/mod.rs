mod ci;

use api_models::{
	models::workspace::infrastructure::deployment::DeploymentStatus,
	utils::{DateTime, Uuid},
};
use axum::{extract::State, response::Response, routing::post, Json, Router};
use chrono::Utc;
use http::{
	header::{AUTHORIZATION, CONTENT_TYPE},
	HeaderMap,
};
use serde_json::json;

use crate::{
	app::App,
	db,
	models::{Action, EventData},
	prelude::*,
	service,
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
pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.route("/docker-registry/notification", post(notification_handler))
		.route("/ci/repo/:repo_id", post(ci::handle_ci_hooks_for_repo))
}

fn docker_registry_error(
	error_code: &str,
	message: &str,
) -> Json<serde_json::Value> {
	Json(json!({
		"errors": [{
			"code": error_code,
			"message": message,
			"detail": []
		}]
	}))
}

async fn notification_handler(
	State(app): State<App>,
	headers: HeaderMap,
	body: String,
) -> Response {
	todo!("need to test registry webhook as compiler is not giving proper suggestions due to other errors");
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {} - Received notification from docker registry",
		request_id,
	);

	log::trace!("request_id: {} - Checking the content type", request_id);
	let content_type_header = headers
		.get(CONTENT_TYPE)
		.and_then(|ct| ct.to_str().ok())
		.unwrap_or_default();
	if content_type_header !=
		"application/vnd.docker.distribution.events.v1+json"
	{
		// Put the different "content-type" header and body in the queue and log
		// the message there
		service::queue_docker_notification_error(
			&body,
			content_type_header,
			&request_id,
		)
		.await
		.map_err(|err| {
			log::error!("Error while added docker msg to queue - {err:?}");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(serde_json::json!({})),
			)
		})?;

		return Ok(StatusCode::ACCEPTED);
	}

	log::trace!(
		"request_id: {} - Checking the Authorization header",
		request_id
	);
	let Some(authorization_header) = headers.get(AUTHORIZATION).and_then(|auth| auth.to_str().ok()) else {
		return Err((StatusCode::UNAUTHORIZED, docker_registry_error(
			"unauthorized",
			"An error occured. If this persists, please contact the administrator"
		)));
	};

	log::trace!(
		"request_id: {} - Parsing the Custom Authorization header",
		request_id
	);
	if authorization_header != app.config.docker_registry.authorization_header {
		return Err((StatusCode::BAD_REQUEST, docker_registry_error(
			"unauthorized",
			"Invalid request sent by the client. Authorization data could not be parsed as expected"
		)));
	}

	let events: EventData = serde_json::from_str(&body)?;

	// check if the event is a push event
	// get image name, repository name, tag if present
	for event in events.events.into_iter().filter(|event| {
		// Only process events that are push events of a manifest
		(event.action == Action::Push || event.action == Action::Mount) &&
			(event.target.media_type ==
				"application/vnd.docker.distribution.manifest.v2+json")
	}) {
		let mut connection = app.database.begin().await?;
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
			&mut connection,
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
			&mut connection,
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
				&app.config,
				&request_id,
			)
			.await?;

			// now process next event
			continue;
		}

		db::create_docker_repository_digest(
			&mut connection,
			&repository.id,
			&target.digest,
			image_size_in_bytes,
			&current_time,
		)
		.await?;

		let total_storage =
			db::get_total_size_of_docker_repositories_for_workspace(
				&mut connection,
				&workspace_id,
			)
			.await?;
		db::update_docker_repo_usage_history(
			&mut connection,
			&workspace_id,
			&(((total_storage as f64) / (1000f64 * 1000f64 * 1000f64)).ceil()
				as i64),
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
			&mut connection,
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
				&mut connection,
				image_name,
				&target.tag,
				&workspace_id,
			)
			.await?;

		connection.commit().await?;

		log::trace!("request_id: {} - Updating the deployments", request_id);
		for db_deployment in deployments {
			let mut connection = app.database.begin().await?;

			if let DeploymentStatus::Stopped = db_deployment.status {
				continue;
			}

			let (deployment, workspace_id, _, deployment_running_details) =
				service::get_full_deployment_config(
					&mut connection,
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
					&mut connection,
					&deployment.id,
					&repository_id,
					&target.digest,
					&current_time,
				)
				.await?;

				db::update_current_live_digest_for_deployment(
					&mut connection,
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
					&mut connection,
					&deployment.registry,
					&deployment.image_tag,
					&app.config,
					&request_id,
				)
				.await?;

			db::update_deployment_status(
				&mut connection,
				&deployment.id,
				&DeploymentStatus::Deploying,
			)
			.await?;

			connection.commit().await?;

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
				&app.config,
				&request_id,
			)
			.await?;
		}
	}

	Ok(StatusCode::ACCEPTED)
}
