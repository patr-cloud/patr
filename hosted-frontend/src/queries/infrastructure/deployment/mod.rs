use leptos_query::*;
use models::api::workspace::deployment::*;

use crate::prelude::*;

/// Tag for Listing All Deployments query
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct AllDeploymentsTag;

/// Query to list all deployments for a workspace
pub fn list_deployments_query(
) -> QueryScope<AllDeploymentsTag, Result<ListDeploymentResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();
	let access_token = state.get().get_access_token();
	let workspace_id = state.get().get_last_used_workspace_id();

	create_query(
		move |_| {
			let access_token = access_token.clone();
			async move {
				list_deployments(
					access_token.clone(),
					workspace_id.clone().map(|id| id.to_string()),
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

	let action = create_action(move |request: &CreateDeploymentRequest| {
		let request = request.clone();
		let navigate = use_navigate();

		let access_token = access_token.clone();

		let deployments_list_query = list_deployments_query();

		async move {
			let response = create_deployment(
				access_token.clone(),
				workspace_id.map(|id| id.to_string()),
				request.clone(),
			)
			.await;

			if let Ok(ref response) = response {
				deployments_list_query.invalidate_query(AllDeploymentsTag);
				navigate(
					format!("/deployments/{}", response.id.id.to_string()).as_str(),
					Default::default(),
				);
			}

			response
		}
	});

	action
}
