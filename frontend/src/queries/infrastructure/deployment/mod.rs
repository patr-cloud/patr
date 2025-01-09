use models::api::workspace::deployment::*;
use time::OffsetDateTime;

use crate::prelude::*;

/// Query to list all deployments for a workspace
pub fn list_deployments_query(
	page: Signal<usize>,
) -> Resource<
	(Option<String>, Option<Uuid>, usize),
	Result<(usize, ListDeploymentResponse), ServerFnError<ErrorType>>,
> {
	let (state, _) = AuthState::load();

	create_resource(
		move || {
			(
				state.get().get_access_token(),
				state.get().get_last_used_workspace_id(),
				page.get(),
			)
		},
		move |(access_token, workspace_id, page)| async move {
			if let Some(workspace_id) = workspace_id {
				list_deployments(
					access_token,
					workspace_id,
					Some(page),
					Some(constants::RESOURCES_PER_PAGE),
				)
				.await
			} else {
				Err(ServerFnError::WrappedServerError(
					ErrorType::WrongParameters,
				))
			}
		},
	)
}

/// Query to get deployment info by id
pub fn get_deployment_query(
	deployment_id: Signal<Uuid>,
) -> Resource<
	(Option<String>, Option<Uuid>, Uuid),
	Result<GetDeploymentInfoResponse, ServerFnError<ErrorType>>,
> {
	let (state, _) = AuthState::load();
	create_resource(
		move || {
			(
				state.get().get_access_token(),
				state.get().get_last_used_workspace_id(),
				deployment_id.get(),
			)
		},
		move |(access_token, workspace_id, deployment_id)| async move {
			get_deployment(access_token, workspace_id, deployment_id).await
		},
	)
}

/// Query to create a deployment, Returns an action to be dispatched on submit.
pub fn create_deployment_query(
) -> Action<CreateDeploymentRequest, Result<CreateDeploymentResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();

	let access_token = state.get().get_access_token();
	let workspace_id = state.get().get_last_used_workspace_id();

	create_action(move |request: &CreateDeploymentRequest| {
		let request = request.clone();
		let navigate = use_navigate();

		let access_token = access_token.clone();

		async move {
			let response = create_deployment(
				access_token.clone(),
				workspace_id.map(|id| id.to_string()),
				request.clone(),
			)
			.await;

			if let Ok(ref response) = response {
				navigate(
					format!("/deployments/{}", response.id.id.to_string()).as_str(),
					Default::default(),
				);
			}

			response
		}
	})
}

/// Query to delete a deployment, Returns an action to be dispatched on submit.
pub fn delete_deployment_query(
) -> Action<Uuid, Result<DeleteDeploymentResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();

	let access_token = state.get().get_access_token();
	let workspace_id = state.get().get_last_used_workspace_id();

	create_action(move |deployment_id: &Uuid| {
		let navigate = use_navigate();
		let access_token = access_token.clone();

		let deployment_id = deployment_id.clone();
		async move {
			let response =
				delete_deployment(access_token.clone(), workspace_id, deployment_id).await;

			if response.is_ok() {
				navigate("/deployments", Default::default());
			}

			response
		}
	})
}

/// Query to start a deployment, Returns an action to be dispatched on submit.
pub fn start_deployment_query(
) -> Action<Uuid, Result<StartDeploymentResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();

	let access_token = state.get().get_access_token();
	let workspace_id = state.get().get_last_used_workspace_id().unwrap();

	create_action(move |deployment_id: &Uuid| {
		let access_token = access_token.clone();

		let deployment_id = deployment_id.clone();

		async move { start_deployment(access_token, workspace_id, deployment_id).await }
	})
}

/// Query to stop a deployment, Returns an action to be dispatched on submit.
pub fn stop_deployment_query(
) -> Action<Uuid, Result<StopDeploymentResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();

	let access_token = state.get().get_access_token();
	let workspace_id = state.get().get_last_used_workspace_id().unwrap();

	create_action(move |deployment_id: &Uuid| {
		let access_token = access_token.clone();

		let deployment_id = deployment_id.clone();

		async move { stop_deployment(access_token, workspace_id, deployment_id).await }
	})
}

/// Query to list all machines for a workspace
pub fn list_machines_query(
) -> Resource<Option<Uuid>, Result<ListAllDeploymentMachineTypeResponse, ServerFnError<ErrorType>>>
{
	create_resource(
		move || AuthState::load().0.get().get_last_used_workspace_id(),
		move |workspace_id| async move { list_all_machines(workspace_id).await },
	)
}

/// Query to get the running logs of a deployment
pub fn get_deployment_logs_query(
	deployment_id: Signal<Uuid>,
	limit: Option<u32>,
	end_time: Signal<Option<OffsetDateTime>>,
) -> Resource<
	(Option<String>, Option<Uuid>, Uuid, Option<OffsetDateTime>),
	Result<GetDeploymentLogsResponse, ServerFnError<ErrorType>>,
> {
	let (state, _) = AuthState::load();

	let access_token = state.get().get_access_token();
	let workspace_id = state.get().get_last_used_workspace_id();

	create_resource(
		move || {
			(
				state.get().get_access_token(),
				state.get().get_last_used_workspace_id(),
				deployment_id.get(),
				end_time.get(),
			)
		},
		move |(access_token, workspace_id, deployment_id, end_time)| async move {
			get_deployment_logs(access_token, workspace_id, deployment_id, end_time, limit).await
		},
	)
}

/// Query to update a deployment, Returns an action to be dispatched on submit.
pub fn update_deployment_query() -> Action<
	(Uuid, UpdateDeploymentRequest),
	Result<UpdateDeploymentResponse, ServerFnError<ErrorType>>,
> {
	let (state, _) = AuthState::load();

	let access_token = state.get().get_access_token();
	let workspace_id = state.get().get_last_used_workspace_id();

	create_action(
		move |(deployment_id, request): &(Uuid, UpdateDeploymentRequest)| {
			let request = request.clone();

			let access_token = access_token.clone();
			let deployment_id = deployment_id.clone();

			async move {
				update_deployment(
					access_token.clone(),
					workspace_id,
					Some(deployment_id),
					request.clone(),
				)
				.await
			}
		},
	)
}
