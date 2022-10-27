use api_macros::closure_as_pinned_box;
use api_models::utils::Uuid;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use reqwest::header;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::permissions,
	pin_fn,
	utils::{
		constants::request_keys,
		settings::MetricsSettings,
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

	sub_app.get(
		"/**",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::metrics::GET,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_metrics_based_on_tenant)),
		],
	);

	sub_app.post(
		"/**",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::metrics::GET,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_metrics_based_on_tenant)),
		],
	);

	sub_app
}

async fn get_metrics_based_on_tenant(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let MetricsSettings {
		host,
		username,
		password,
	} = &context.get_state().config.metrics;

	let hyper_request = context.get_request().get_hyper_request();

	let path = {
		let path = hyper_request.uri().path();

		// /workspace/cc36ed2cf780429baeef4b80490d2277/metrics/**
		// |-------------------------------------------------|
		//                51 total characters
		//
		// its safe to skip 51 chars, as the route will come here only if those
		// thing were matched
		let trimmed_path = &path[11..]; // strip `/workspace/`
		let (_, trimmed_path) = trimmed_path.split_once('/').unwrap(); // strip workspace_id
		let trimmed_path = &trimmed_path[7..]; // strip `metrics`

		format!("https://{host}{trimmed_path}")
	};

	let query_params = {
		let mut query_params = hyper_request
			.uri()
			.query()
			.map(querystring::querify)
			.unwrap_or_default()
			.into_iter()
			.filter(|(key, _)| key.to_lowercase() == "namespace")
			.collect::<Vec<_>>();

		query_params.push(("namespace", workspace_id.as_str()));

		query_params
	};

	let basic_headers = hyper_request
		.headers()
		.iter()
		.filter_map(|(hn, hv)| match hn {
			&header::ACCEPT |
			&header::ACCEPT_CHARSET |
			&header::ACCEPT_ENCODING |
			&header::ALLOW |
			&header::CONTENT_LENGTH |
			&header::CONTENT_ENCODING |
			&header::CONTENT_TYPE => Some((hn.clone(), hv.clone())),
			_ => None,
		})
		.collect();

	let response = reqwest::Client::new()
		.request(hyper_request.method().clone(), path)
		.query(&query_params)
		.headers(basic_headers)
		.basic_auth(username, Some(password))
		.header(header::USER_AGENT, "Patr")
		.body(context.get_request().get_body_bytes().to_owned())
		.send()
		.await?;

	// return the result of above http call
	context.status(response.status().as_u16());
	for (header_name, header_value) in response.headers() {
		context.append_header(header_name.as_str(), header_value.to_str()?);
	}

	let body = response.bytes().await?;
	context.get_response_mut().set_body_bytes(&body);

	Ok(context)
}
