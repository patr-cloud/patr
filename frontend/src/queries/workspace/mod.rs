use models::api::{user::ListUserWorkspacesResponse, workspace::GetWorkspaceInfoResponse};

use crate::{get_workspace_info, list_user_workspace, prelude::*};

/// Query to list all workspaces
pub fn list_workspaces_query(
) -> Resource<Option<String>, Result<ListUserWorkspacesResponse, ServerFnError<ErrorType>>> {
	create_resource(
		move || AuthState::load().0.get().get_access_token(),
		move |(access_token)| async move {
			if let Some(access_token) = access_token {
				list_user_workspace(access_token).await
			} else {
				Err(ServerFnError::WrappedServerError(ErrorType::Unauthorized))
			}
		},
	)
}

/// Query to get a workspace
pub fn get_workspace_query(
	workspace_id: Signal<Uuid>,
) -> Resource<(Option<String>, Uuid), Result<GetWorkspaceInfoResponse, ServerFnError<ErrorType>>> {
	create_resource(
		move || {
			(
				AuthState::load().0.get().get_access_token(),
				workspace_id.get(),
			)
		},
		move |(access_token, workspace_id)| async move {
			get_workspace_info(access_token, workspace_id).await
		},
	)
}
