use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::secret::{
		CreateSecretInWorkspaceRequest,
		CreateSecretInWorkspaceResponse,
		DeleteSecretResponse,
		ListSecretsResponse,
		Secret,
		UpdateWorkspaceSecretRequest,
		UpdateWorkspaceSecretResponse,
	},
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use zeroize::Zeroize;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{rbac::permissions, ResourceType},
	pin_fn,
	service,
	utils::{constants::request_keys, Error, EveContext, EveMiddleware},
};

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, Error> {
	let mut app = create_eve_app(app);

	// List all secrets
	app.get(
		"/",
		[
			EveMiddleware::WorkspaceMemberAuthenticator {
				is_api_token_allowed: true,
				requested_workspace: closure_as_pinned_box!(|context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					Ok((context, workspace_id))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(list_secrets)),
		],
	);

	// Create a new secret
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::secret::CREATE,
				resource: closure_as_pinned_box!(|mut context| {
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
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(create_secret)),
		],
	);

	// update secret
	app.patch(
		"/:secretId",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::secret::EDIT,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let secret_id =
						context.get_param(request_keys::SECRET_ID).unwrap();
					let secret_id = Uuid::parse_str(secret_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&secret_id,
					)
					.await?
					.filter(|resource| resource.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(update_secret)),
		],
	);

	// delete secret
	app.delete(
		"/:secretId",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::secret::DELETE,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let secret_id =
						context.get_param(request_keys::SECRET_ID).unwrap();
					let secret_id = Uuid::parse_str(secret_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&secret_id,
					)
					.await?
					.filter(|resource| resource.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(delete_secret)),
		],
	);

	app
}

async fn list_secrets(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - Listing all secrets", request_id);
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let user_token = context.get_token_data().status(500)?.clone();

	let secrets = db::get_all_secrets_in_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.filter(|secret| {
		user_token.has_access_for_requested_action(
			&workspace_id,
			&secret.id,
			permissions::workspace::secret::INFO,
		)
	})
	.map(|secret| Secret {
		id: secret.id,
		name: secret.name,
		deployment_id: secret.deployment_id,
	})
	.collect();

	log::trace!("request_id: {} - Returning secrets", request_id);
	context.success(ListSecretsResponse { secrets }).await?;
	Ok(context)
}

async fn create_secret(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let CreateSecretInWorkspaceRequest {
		name, mut value, ..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();

	log::trace!("{} - Creating new secret {}", request_id, workspace_id,);
	let id = service::create_new_secret_in_workspace(
		context.get_database_connection(),
		&workspace_id,
		&name,
		&value,
		&config,
		&request_id,
	)
	.await?;

	value.zeroize();

	log::trace!("request_id: {} - Returning new secret", request_id);
	context
		.success(CreateSecretInWorkspaceResponse { id })
		.await?;
	Ok(context)
}

async fn update_secret(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let secret_id =
		Uuid::parse_str(context.get_param(request_keys::SECRET_ID).unwrap())
			.unwrap();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let UpdateWorkspaceSecretRequest { name, value, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();

	log::trace!("request_id: {} - Deleting secret {}", request_id, secret_id);
	service::update_workspace_secret(
		context.get_database_connection(),
		&workspace_id,
		&secret_id,
		name.as_deref(),
		value.as_deref(),
		&config,
		&request_id,
	)
	.await?;

	if let Some(mut value) = value {
		value.zeroize();
	}

	context.success(UpdateWorkspaceSecretResponse {}).await?;
	Ok(context)
}

async fn delete_secret(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let user_id = context.get_token_data().unwrap().user_id().clone();

	let secret_id =
		Uuid::parse_str(context.get_param(request_keys::SECRET_ID).unwrap())
			.unwrap();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let secret =
		db::get_secret_by_id(context.get_database_connection(), &secret_id)
			.await?
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let config = context.get_state().config.clone();

	log::trace!("request_id: {} - Deleting secret {}", request_id, secret_id);
	service::delete_secret_in_workspace(
		context.get_database_connection(),
		&workspace_id,
		&secret_id,
		&config,
		&request_id,
	)
	.await?;

	// Commiting transaction so that even if the mailing function fails the
	// resource should be deleted
	context.commit_database_transaction().await?;

	service::resource_delete_action_email(
		context.get_database_connection(),
		&secret.name,
		&secret.workspace_id,
		&ResourceType::Secret,
		&user_id,
	)
	.await?;

	context.success(DeleteSecretResponse {}).await?;
	Ok(context)
}
