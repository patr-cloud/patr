use models::api::{user::*, workspace::rbac::ListAllPermissionsResponse};

use crate::prelude::*;

/// Query to list all API tokens
pub fn list_api_tokens_query(
) -> Resource<Option<String>, Result<ListApiTokensResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();
	let access_token = state.get().get_access_token();

	create_resource(
		move || state.get().get_access_token(),
		move |access_token| async move { load_api_tokens_list(access_token).await },
	)
}

/// Query to get a single API token
pub fn get_api_token_query(
	token_id: Signal<Uuid>,
) -> Resource<(Option<String>, Uuid), Result<GetApiTokenInfoResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();

	create_resource(
		move || (state.get().get_access_token(), token_id.get()),
		move |(access_token, token_id)| async move { get_api_token(access_token, token_id).await },
	)
}

/// Query to get all permissions
pub fn get_all_permissions_query() -> Resource<
	(Option<String>, Option<Uuid>),
	Result<ListAllPermissionsResponse, ServerFnError<ErrorType>>,
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
				list_all_permissions(access_token, workspace_id).await
			} else {
				Err(ServerFnError::WrappedServerError(ErrorType::Unauthorized))
			}
		},
	)
}

/// Query to create a new API token
pub fn create_api_token_query(
) -> Action<CreateApiTokenRequest, Result<CreateApiTokenResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();
	let access_token = state.get().get_access_token();

	create_action(move |request: &CreateApiTokenRequest| {
		let request = request.clone();
		let access_token = access_token.clone();

		async move { create_api_token(access_token.clone(), request.clone()).await }
	})
}
