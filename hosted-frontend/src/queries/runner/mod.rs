use leptos_query::*;
use models::api::workspace::runner::*;

use crate::prelude::*;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct AllRunnersTag;

pub fn list_runners_query(
) -> QueryScope<AllRunnersTag, Result<ListRunnersForWorkspaceResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();
	let access_token = state.get().get_access_token();
	let workspace_id = state.get().get_last_used_workspace_id();

	create_query(
		move |_| {
			let access_token = access_token.clone();
			async move {
				list_runners(
					workspace_id.clone().map(|id| id.to_string()),
					access_token.clone(),
				)
				.await
			}
		},
		QueryOptions {
			..Default::default()
		},
	)
}

pub fn get_runners_query(
) -> QueryScope<Uuid, Result<GetRunnerInfoResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();
	let access_token = state.get().get_access_token();
	let workspace_id = state.get().get_last_used_workspace_id();

	create_query(
		move |_, runner_id| {
			let access_token = access_token.clone();
			async move { get_runner(access_token, workspace_id, runner_id).await }
		},
		QueryOptions {
			..Default::default()
		},
	)
}
