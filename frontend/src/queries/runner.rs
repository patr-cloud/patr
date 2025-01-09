use models::api::workspace::runner::*;

use crate::prelude::*;

/// Query to list all runners for a workspace
pub fn list_runners_query() -> Resource<
	(Option<String>, Option<Uuid>),
	Result<ListRunnersForWorkspaceResponse, ServerFnError<ErrorType>>,
> {
	let (state, _) = AuthState::load();

	create_resource(
		move || {
			(
				state.get().get_access_token(),
				state.get().get_last_used_workspace_id(),
			)
		},
		move |(access_token, workspace_id)| async move {
			if let Some(workspace_id) = workspace_id {
				list_runners(access_token, workspace_id).await
			} else {
				Err(ServerFnError::WrappedServerError(ErrorType::Unauthorized))
			}
		},
	)
}

/// Query to get a runner by id
pub fn get_runner_query(
	runner_id: Signal<Uuid>,
) -> Resource<
	(Option<String>, Option<Uuid>, Uuid),
	Result<GetRunnerInfoResponse, ServerFnError<ErrorType>>,
> {
	let (state, _) = AuthState::load();
	create_resource(
		move || {
			(
				state.get().get_access_token(),
				state.get().get_last_used_workspace_id(),
				runner_id.get(),
			)
		},
		move |(access_token, workspace_id, runner_id)| async move {
			get_runner(access_token, workspace_id, runner_id).await
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

		async move {
			let response = create_runner(access_token, workspace_id, runner_name).await;

			if let Ok(ref response) = response {
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

		async move {
			let response = delete_runner(access_token, workspace_id, runner_id).await;

			if let Ok(_) = response {
				navigate("/runners", Default::default());
			}

			response
		}
	})
}
