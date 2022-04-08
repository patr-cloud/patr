use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::infrastructure::secret::{
		CreateSecretForDeploymentRequest,
		CreateSecretForDeploymentResponse,
		CreateSecretRequest,
		CreateSecretResponse,
		DeleteSecretResponse,
		ListSecretsResponse,
		Secret,
	},
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::permissions,
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

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut app = create_eve_app(app);

	// List all secrets
	app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::secret::LIST,
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
			EveMiddleware::CustomFunction(pin_fn!(list_secrets)),
		],
	);

	// Create a new secret
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::secret::CREATE,
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
			EveMiddleware::CustomFunction(pin_fn!(create_secret)),
		],
	);

	app.post(
		"/:deploymentId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::secret::CREATE,
				closure_as_pinned_box!(|mut context| {
					let deployment_id =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = Uuid::parse_str(deployment_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
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
			EveMiddleware::CustomFunction(pin_fn!(
				create_secret_for_deployment
			)),
		],
	);

	// delete secret
	app.post(
		"/:secretId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::secret::DELETE,
				closure_as_pinned_box!(|mut context| {
					let secret_id =
						context.get_param(request_keys::SECRET_ID).unwrap();
					let secret_id = Uuid::parse_str(secret_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&secret_id,
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
			EveMiddleware::CustomFunction(pin_fn!(delete_secret)),
		],
	);

	app
}

async fn list_secrets(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - Listing all secrets", request_id);
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let secrets = db::get_all_secrets_in_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.map(|secret| Secret {
		id: secret.id,
		name: secret.name,
		deployment_id: secret.deployment_id,
	})
	.collect();

	log::trace!("request_id: {} - Returning secrets", request_id);
	context.success(ListSecretsResponse { secrets });
	Ok(context)
}

async fn create_secret(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let CreateSecretRequest { name, secret, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();

	log::trace!("{} - Creating new secret {}", request_id, workspace_id,);
	let id = service::create_new_secret_in_workspace(
		context.get_database_connection(),
		&workspace_id,
		// deployment_id.as_ref(),
		&name,
		&secret,
		&config,
		&request_id,
	)
	.await?;

	log::trace!("request_id: {} - Returning new secret", request_id);
	context.success(CreateSecretResponse { id });
	Ok(context)
}

async fn create_secret_for_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let deployment_id = Uuid::parse_str(
		context.get_param(request_keys::DEPLOYMENT_ID).unwrap(),
	)
	.unwrap();

	log::trace!(
		"{} - Creating new secret in deployment - {}",
		request_id,
		deployment_id
	);

	let CreateSecretForDeploymentRequest { name, secret, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();

	let id = service::create_new_secret_for_deployment(
		context.get_database_connection(),
		&workspace_id,
		&deployment_id,
		&name,
		&secret,
		&config,
		&request_id,
	)
	.await?;

	log::trace!("request_id: {} - Returning new secret", request_id);
	context.success(CreateSecretForDeploymentResponse { id });
	Ok(context)
}

async fn delete_secret(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let secret_id =
		Uuid::parse_str(context.get_param(request_keys::SECRET_ID).unwrap())
			.unwrap();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let config = context.get_state().config.clone();

	log::trace!(
		"request_id: {} - Deleting managed URL {}",
		request_id,
		secret_id
	);
	service::delete_secret_in_workspace(
		context.get_database_connection(),
		&workspace_id,
		&secret_id,
		&config,
		&request_id,
	)
	.await?;

	context.success(DeleteSecretResponse {});
	Ok(context)
}
