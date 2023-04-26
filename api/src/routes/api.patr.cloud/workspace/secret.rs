use api_macros::closure_as_pinned_box;
use api_models::{
	models::prelude::*,
	utils::{DecodedRequest, Paginated, Uuid},
};
use axum::{extract::State, Extension};
use zeroize::Zeroize;

use crate::{
	app::App,
	db,
	error,
	models::{rbac::permissions, ResourceType, UserAuthenticationData},
	prelude::*,
	service,
	utils::{constants::request_keys, Error},
};

pub fn create_sub_app(app: &App) -> Router<App> {
	let mut app = create_axum_router(app);

	// List all secrets
	app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::secret::LIST,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
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

					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
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

					let resource =
						db::get_resource_by_id(&mut connection, &secret_id)
							.await?
							.filter(|resource| {
								resource.owner_id == workspace_id
							});

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
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

					let resource =
						db::get_resource_by_id(&mut connection, &secret_id)
							.await?
							.filter(|resource| {
								resource.owner_id == workspace_id
							});

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
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
	mut connection: Connection,
	Extension(token_data): Extension<UserAuthenticationData>,
	State(config): State<Config>,
	DecodedRequest {
		path: ListSecretsPath { workspace_id },
		query: Paginated {
			count: _,
			start: _,
			query: (),
		},
		body: (),
	}: DecodedRequest<ListSecretsRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - Listing all secrets", request_id);
	let secrets =
		db::get_all_secrets_in_workspace(&mut connection, &workspace_id)
			.await?
			.into_iter()
			.map(|secret| Secret {
				id: secret.id,
				name: secret.name,
				deployment_id: secret.deployment_id,
			})
			.collect();

	log::trace!("request_id: {} - Returning secrets", request_id);
	Ok(ListSecretsResponse { secrets })
}

async fn create_secret(
	mut connection: Connection,
	Extension(token_data): Extension<UserAuthenticationData>,
	State(config): State<Config>,
	DecodedRequest {
		path: CreateSecretInWorkspacePath { workspace_id },
		query: (),
		body: CreateSecretInWorkspaceRequest { name, value },
	}: DecodedRequest<CreateSecretInWorkspaceRequest>,
) -> Result<CreateSecretInWorkspaceResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("{} - Creating new secret {}", request_id, workspace_id,);
	let id = service::create_new_secret_in_workspace(
		&mut connection,
		&workspace_id,
		&name,
		&value,
		&config,
		&request_id,
	)
	.await?;

	value.zeroize();

	log::trace!("request_id: {} - Returning new secret", request_id);
	Ok(CreateSecretInWorkspaceResponse { id })
}

async fn update_secret(
	mut connection: Connection,
	Extension(token_data): Extension<UserAuthenticationData>,
	State(config): State<Config>,
	DecodedRequest {
		path: DeleteSecretPath {
			workspace_id,
			secret_id,
		},
		query: (),
		body: UpdateWorkspaceSecretRequest { name, value },
	}: DecodedRequest<UpdateWorkspaceSecretRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - Updating secret {}", request_id, secret_id);
	service::update_workspace_secret(
		&mut connection,
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

	Ok(())
}

async fn delete_secret(
	mut connection: Connection,
	Extension(token_data): Extension<UserAuthenticationData>,
	State(config): State<Config>,
	DecodedRequest {
		path: DeleteSecretPath {
			workspace_id,
			secret_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<DeleteSecretRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	let user_id = token_data.user_id();

	let secret = db::get_secret_by_id(&mut connection, &secret_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	log::trace!("request_id: {} - Deleting secret {}", request_id, secret_id);
	service::delete_secret_in_workspace(
		&mut connection,
		&workspace_id,
		&secret_id,
		&config,
		&request_id,
	)
	.await?;

	// Commiting transaction so that even if the mailing function fails the
	// resource should be deleted
	connection.commit().await?;

	service::resource_delete_action_email(
		&mut connection,
		&secret.name,
		&secret.workspace_id,
		&ResourceType::Secret,
		&user_id,
	)
	.await?;

	Ok(())
}
