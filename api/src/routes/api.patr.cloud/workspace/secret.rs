use api_models::{
	models::prelude::*,
	utils::{DecodedRequest, Paginated, Uuid},
};
use axum::{extract::State, Extension, Router};
use zeroize::Zeroize;

use crate::{
	app::App,
	db,
	models::{rbac::permissions, ResourceType, UserAuthenticationData},
	prelude::*,
	service,
	utils::Error,
};

pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::secret::LIST,
				|ListSecretsPath { workspace_id },
				 Paginated {
				     count: _,
				     start: _,
				     query: (),
				 },
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			list_secrets,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::secret::CREATE,
				|CreateSecretInWorkspacePath { workspace_id },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			create_secret,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::secret::EDIT,
				|DeleteSecretPath {
				     workspace_id,
				     secret_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &secret_id)
						.await?
						.filter(|resource| resource.owner_id == workspace_id)
				},
			),
			app.clone(),
			update_secret,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::secret::DELETE,
				|DeleteSecretPath {
				     workspace_id,
				     secret_id,
				 },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &secret_id)
						.await?
						.filter(|resource| resource.owner_id == workspace_id)
				},
			),
			app.clone(),
			delete_secret,
		)
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
		path: UpdateWorkspaceSecretPath {
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
