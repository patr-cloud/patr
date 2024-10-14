use leptos_query::*;
use models::api::{user::ListUserWorkspacesResponse, workspace::GetWorkspaceInfoResponse};

use crate::{get_workspace_info, list_user_workspace, prelude::*};

/// Tag for Listing All Workspaces query
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct AllWorkspacesTag;

/// Query to list all workspaces
pub fn list_workspaces_query(
) -> QueryScope<AllWorkspacesTag, Result<ListUserWorkspacesResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();
	let access_token = state.get().get_access_token();

	create_query(
		move |_| {
			let access_token = access_token.clone();
			async move { list_user_workspace(access_token.clone()).await }
		},
		QueryOptions {
			..Default::default()
		},
	)
}

/// Query to get a workspace
pub fn get_workspace_query(
) -> QueryScope<Uuid, Result<GetWorkspaceInfoResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();
	let access_token = state.get().get_access_token();

	create_query(
		move |workspace_id: Uuid| {
			let access_token = access_token.clone();
			async move { get_workspace_info(access_token.clone(), workspace_id).await }
		},
		QueryOptions {
			..Default::default()
		},
	)
}
