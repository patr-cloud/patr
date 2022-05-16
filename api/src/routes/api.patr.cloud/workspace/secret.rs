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
	models::{
		rbac::{self, permissions},
		SecretMetaData,
	},
	pin_fn,
	service,
	utils::{
		constants::request_keys,
		AuditLogData,
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
				permissions::workspace::secret::LIST,
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
				permissions::workspace::secret::CREATE,
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
			EveMiddleware::WorkspaceAuditLogger,
			EveMiddleware::CustomFunction(pin_fn!(create_secret)),
		],
	);

	// update secret
	app.patch(
		"/:secretId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::secret::EDIT,
				closure_as_pinned_box!(|mut context| {
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
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::WorkspaceAuditLogger,
			EveMiddleware::CustomFunction(pin_fn!(update_secret)),
		],
	);

	// delete secret
	app.delete(
		"/:secretId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::secret::DELETE,
				closure_as_pinned_box!(|mut context| {
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
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::WorkspaceAuditLogger,
			EveMiddleware::CustomFunction(pin_fn!(delete_secret)),
		],
	);

	app
}

async fn list_secrets(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = context.get_request_id().clone();

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
	let request_id = context.get_request_id().clone();
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

	context.set_audit_log_data(AuditLogData {
		resource_id: id.clone(),
		action_id: rbac::PERMISSIONS
			.get()
			.and_then(|map| {
				map.get(permissions::workspace::secret::CREATE).cloned()
			})
			.unwrap(),
		metadata: Some(serde_json::to_value(SecretMetaData::Create)?),
	});

	log::trace!("request_id: {} - Returning new secret", request_id);
	context.success(CreateSecretInWorkspaceResponse { id });
	Ok(context)
}

async fn update_secret(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = context.get_request_id().clone();

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

	context.set_audit_log_data(AuditLogData {
		resource_id: secret_id.clone(),
		action_id: rbac::PERMISSIONS
			.get()
			.and_then(|map| {
				map.get(permissions::workspace::secret::EDIT).cloned()
			})
			.unwrap(),
		metadata: Some(serde_json::to_value(SecretMetaData::Edit)?),
	});

	context.success(UpdateWorkspaceSecretResponse {});
	Ok(context)
}

async fn delete_secret(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = context.get_request_id().clone();

	let secret_id =
		Uuid::parse_str(context.get_param(request_keys::SECRET_ID).unwrap())
			.unwrap();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

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

	context.set_audit_log_data(AuditLogData {
		resource_id: secret_id.clone(),
		action_id: rbac::PERMISSIONS
			.get()
			.and_then(|map| {
				map.get(permissions::workspace::secret::DELETE).cloned()
			})
			.unwrap(),
		metadata: Some(serde_json::to_value(SecretMetaData::Delete)?),
	});

	context.success(DeleteSecretResponse {});
	Ok(context)
}
