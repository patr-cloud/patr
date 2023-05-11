use api_models::{models::prelude::*, utils::DateTime};
use axum::{extract::State, Router};

use crate::{
	app::App,
	db,
	models::rbac::permissions,
	prelude::*,
	utils::Error,
};

mod github;

pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::ci::git_provider::LIST,
				|ListGitProvidersPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			)
			.disallow_api_token(),
			app.clone(),
			list_git_providers,
		)
		.merge(github::create_sub_app(app))
}

async fn list_git_providers(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: ListGitProvidersPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<ListGitProvidersRequest>,
) -> Result<ListGitProvidersResponse, Error> {
	let git_providers = db::list_connected_git_providers_for_workspace(
		&mut connection,
		&workspace_id,
	)
	.await?
	.into_iter()
	.map(|git_provider| GitProvider {
		id: git_provider.id,
		domain_name: git_provider.domain_name,
		git_provider_type: api_models::models::workspace::ci::git_provider::GitProviderType::Github,
		login_name: git_provider.login_name,
		is_syncing: git_provider.is_syncing,
		last_synced: git_provider.last_synced.map(DateTime),
		is_deleted: git_provider.is_deleted
	})
	.collect();

	Ok(ListGitProvidersResponse { git_providers });
}
