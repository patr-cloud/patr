use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use serde_json::json;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::db_mapping::EventData,
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
// TODO: add custom header for this endpoint
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
		Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
	}
	let body = context.get_body()?;
	let events: EventData = serde_json::from_str(&body)?;

	// check if the event is a push event
	// get image name, repository name, tag if present
	for event in events.events {
		if event.action != "push" {
			continue;
		}
		let target = event.target;
		if target.tag.is_empty() {
			continue;
		}

		let repository = target.repository;
		let mut splitter = repository.split('/');
		let org_name = if let Some(val) = splitter.next() {
			val
		} else {
			continue;
		};
		let image_name = if let Some(val) = splitter.next() {
			val
		} else {
			continue;
		};
		let tag = target.tag;

		let organisation = db::get_organisation_by_name(
			context.get_database_connection(),
			org_name,
		)
		.await?;
		let organisation = if let Some(organisation) = organisation {
			organisation
		} else {
			continue;
		};

		let deployments =
			db::get_deployments_by_image_name_and_tag_for_organisation(
				context.get_database_connection(),
				image_name,
				&tag,
				&organisation.id,
			)
			.await?;

		for deployment in deployments {
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

			let config = context.get_state().config.clone();

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
