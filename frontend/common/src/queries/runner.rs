use models::api::workspace::runner::*;

use crate::{api::list_runners, prelude::*};
/// Query to list all runners for a workspace
pub fn list_runners_query() -> Resource<Result<ListRunnersForWorkspaceResponse, AppError>> {
	let (state, _) = AuthState::load();
	info!("{:#?}", state.get());
	Resource::new(
		move || {
			(
				state.get().get_access_token(),
				state.get().get_last_used_workspace_id(),
			)
		},
		move |(access_token, workspace_id)| async move {
			info!("{:#?}, {:#?}", workspace_id, access_token);
			if let Some(workspace_id) = workspace_id {
				list_runners(access_token, workspace_id).await
			} else {
				Err(AppError::General("Api error from query".to_string()))
			}
		},
	)
}
