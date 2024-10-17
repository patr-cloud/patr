use leptos_query::*;
use models::api::workspace::deployment::*;
use time::OffsetDateTime;

use crate::prelude::*;

/// Tag for Listing All Deployments query
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct AllDeploymentsTag(pub usize);

/// Query to list all deployments for a workspace
pub fn list_deployments_query(
) -> QueryScope<AllDeploymentsTag, Result<(usize, ListDeploymentResponse), ServerFnError<ErrorType>>>
{
	let (state, _) = AuthState::load();
	let access_token = state.get().get_access_token();
	// TODO: remove this unwrap
	let workspace_id = state.get().get_last_used_workspace_id().unwrap();

	logging::log!("here i'm being called here");
	create_query(
		move |query_tag: AllDeploymentsTag| {
			let access_token = access_token.clone();
			async move {
				list_deployments(
					access_token,
					workspace_id,
					Some(query_tag.0),
					Some(constants::RESOURCES_PER_PAGE),
				)
				.await
			}
		},
		QueryOptions {
			..Default::default()
		},
	)
}

/// Query to get deployment info by id
pub fn get_deployment_query(
) -> QueryScope<Uuid, Result<GetDeploymentInfoResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();
	let access_token = state.get().get_access_token();
	let workspace_id = state.get().get_last_used_workspace_id();

	create_query(
		move |deployment_id: Uuid| {
			let access_token = access_token.clone();
			async move { get_deployment(access_token.clone(), workspace_id, deployment_id).await }
		},
		QueryOptions {
			..Default::default()
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
				let _ = use_query_client().invalidate_query_type::<AllDeploymentsTag, Result<ListDeploymentResponse, ServerFnError<ErrorType>>>();

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

		let deployment_query = get_deployment_query();

		async move {
			let _ = deployment_query.cancel_query(deployment_id);

			let response =
				delete_deployment(access_token.clone(), workspace_id, deployment_id).await;

			if response.is_ok() {
				let _ = deployment_query.invalidate_query(deployment_id);
				let _ = use_query_client().invalidate_query_type::<AllDeploymentsTag, Result<ListDeploymentResponse, ServerFnError<ErrorType>>>();

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
		let deployment_query = get_deployment_query();

		async move {
			let response = start_deployment(access_token, workspace_id, deployment_id).await;
			let _ = deployment_query.invalidate_query(deployment_id);

			response
		}
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
		let deployment_query = get_deployment_query();

		async move {
			let response = stop_deployment(access_token, workspace_id, deployment_id).await;
			let _ = deployment_query.invalidate_query(deployment_id);

			response
		}
	})
}

/// Tag for Listing All Deployments query
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct AllMachinesTag;

/// Query to list all machines for a workspace
pub fn list_machines_query() -> QueryScope<
	AllMachinesTag,
	Result<ListAllDeploymentMachineTypeResponse, ServerFnError<ErrorType>>,
> {
	let (state, _) = AuthState::load();
	let workspace_id = state.get().get_last_used_workspace_id().unwrap();

	create_query(
		move |_| async move { list_all_machines(workspace_id).await },
		QueryOptions {
			..Default::default()
		},
	)
}

/// Tag for Getting the initial set of logs for a deployment
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct GetDeploymentLogsTag(pub Uuid);

/// Query to get the running logs of a deployment
pub fn get_deployment_logs_query(
) -> QueryScope<GetDeploymentLogsTag, Result<GetDeploymentLogsResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();

	let access_token = state.get().get_access_token();
	let workspace_id = state.get().get_last_used_workspace_id();

	create_query(
		move |deployment_id: GetDeploymentLogsTag| {
			let access_token = access_token.clone();
			let deployment_id = deployment_id.0;
			async move {
				get_deployment_logs(
					access_token.clone(),
					workspace_id,
					deployment_id,
					Some(OffsetDateTime::now_utc()),
					Some(100),
				)
				.await
			}
		},
		QueryOptions {
			..Default::default()
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

			let deployments_list_query = list_deployments_query();
			let deployment_query = get_deployment_query();

			async move {
				let _ = deployment_query.cancel_query(deployment_id);

				let response = update_deployment(
					access_token.clone(),
					workspace_id,
					Some(deployment_id),
					request.clone(),
				)
				.await;

				if response.is_ok() {
					let _ = use_query_client().invalidate_query_type::<AllDeploymentsTag, Result<ListDeploymentResponse, ServerFnError<ErrorType>>>();
					let _ = deployment_query.invalidate_query(deployment_id);
				}

				response
			}
		},
	)
}
