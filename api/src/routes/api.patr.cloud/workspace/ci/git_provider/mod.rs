use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::ci::git_provider::{
		GitProvider,
		ListGitProvidersResponse,
	},
	utils::{DateTime, Uuid},
};
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

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut sub_app = create_eve_app(app);

	sub_app.use_sub_app("/github", github::create_sub_app(app));

	sub_app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission: permissions::workspace::ci::git_provider::LIST,
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
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(list_git_providers)),
		],
	);

	sub_app
}

// TODO do we need this route anymore if we have no concept of workspace for
// github app, it is about user?
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
		// hard-coding this value now, have to make it dynamic once other ci_providers are introduced
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
