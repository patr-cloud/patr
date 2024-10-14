use leptos_query::*;
use models::api::workspace::runner::*;

use crate::prelude::*;

/// Tag for Listing All Runners query
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct AllRunnersTag;

/// Query to list all runners for a workspace
pub fn list_runners_query(
) -> QueryScope<AllRunnersTag, Result<ListRunnersForWorkspaceResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();
	let access_token = state.get().get_access_token();
	// TODO remove this unwrap
	let workspace_id = state.get().get_last_used_workspace_id().unwrap();

	create_query(
		move |_| {
			let access_token = access_token.clone();
			async move { list_runners(workspace_id, access_token.clone()).await }
		},
		QueryOptions {
			..Default::default()
		},
	)
}

/// Query to get a runner by id
pub fn get_runner_query(
) -> QueryScope<Uuid, Result<GetRunnerInfoResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();
	let access_token = state.get().get_access_token();
	let workspace_id = state.get().get_last_used_workspace_id();

	create_query(
		move |runner_id| {
			let access_token = access_token.clone();
			async move { get_runner(access_token, workspace_id, runner_id).await }
		},
		QueryOptions {
			..Default::default()
		},
	)
}

/// Query to create a runner, Returns an action to be dispatched on submit.
/// The action will navigate to the created runner and invalidate the runners
/// list.
pub fn create_runner_query(
) -> Action<String, Result<AddRunnerToWorkspaceResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();

	let access_token = state.get().get_access_token();
	let workspace_id = state.get().get_last_used_workspace_id();

	create_action(move |runner_name: &String| {
		let navigate = use_navigate();

		let access_token = access_token.clone();
		let runner_name = runner_name.clone();

		let runners_list_query = list_runners_query();

		async move {
			let response = create_runner(access_token, workspace_id, runner_name).await;

			if let Ok(ref response) = response {
				runners_list_query.invalidate_query(AllRunnersTag);
				navigate(
					format!("/runners/{}", response.id.id.to_string()).as_str(),
					Default::default(),
				);
			}

			response
		}
	})
}

/// Query to delete a runner, Returns an action to be dispatched on submit.
/// The action will navigate to the runners list and invalidate the runners
/// list and the runner.
pub fn delete_runner_query() -> Action<Uuid, Result<DeleteRunnerResponse, ServerFnError<ErrorType>>>
{
	let (state, _) = AuthState::load();

	let access_token = state.get().get_access_token();
	let workspace_id = state.get().get_last_used_workspace_id();

	create_action(move |runner_id: &Uuid| {
		let navigate = use_navigate();

		let access_token = access_token.clone();
		let runner_id = runner_id.clone();

		let runners_list_query = list_runners_query();
		let runner_query = get_runner_query();

		async move {
			let response = delete_runner(access_token, workspace_id, runner_id).await;

			if let Ok(_) = response {
				let _ = runners_list_query.invalidate_query(AllRunnersTag);
				let _ = runner_query.invalidate_query(runner_id);

				navigate("/runners", Default::default());
			}

			response
		}
	})
}
