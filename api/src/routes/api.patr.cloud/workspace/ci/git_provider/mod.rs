use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::ci::git_provider::{
		GitProvider,
		ListGitProvidersResponse,
	},
	utils::{DateTime, Uuid},
};
use axum::{routing::get, Router};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::permissions,
	pin_fn,
	utils::{
		constants::request_keys,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};

mod github;

pub fn create_sub_route(app: &App) -> Router {
	let router = Router::new();

	router.nest("/github", github::create_sub_route(app));

	route.route("/", get(list_git_providers));

	sub_app
}

async fn list_git_providers(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let git_providers = db::list_connected_git_providers_for_workspace(
		context.get_database_connection(),
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

	context.success(ListGitProvidersResponse { git_providers });
	Ok(context)
}
