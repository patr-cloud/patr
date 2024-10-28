use leptos_query::*;
use models::api::{user::*, workspace::rbac::ListAllPermissionsResponse};

use crate::prelude::*;

/// Tag for Listing All API Tokens query
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct AllApiTokensTag;

/// Query to list all API tokens
pub fn list_api_tokens_query(
) -> QueryScope<AllApiTokensTag, Result<ListApiTokensResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();
	let access_token = state.get().get_access_token();

	create_query(
		move |_| {
			let access_token = access_token.clone();
			async move { load_api_tokens_list(access_token.clone()).await }
		},
		QueryOptions {
			..Default::default()
		},
	)
}

/// Query to get a single API token
pub fn get_api_token_query(
) -> QueryScope<Uuid, Result<GetApiTokenInfoResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();
	let access_token = state.get().get_access_token();

	create_query(
		move |token_id| {
			let access_token = access_token.clone();
			async move { get_api_token(access_token.clone(), token_id).await }
		},
		QueryOptions {
			..Default::default()
		},
	)
}

pub fn get_all_permissions_query(
) -> QueryScope<Uuid, Result<ListAllPermissionsResponse, ServerFnError<ErrorType>>> {
	let (state, _) = AuthState::load();

	create_query(
		move |workspace_id| {
			let access_token = state.get().get_access_token();
			async move { list_all_permissions(access_token, workspace_id).await }
		},
		Default::default(),
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

		let api_tokens_list_query = list_api_tokens_query();

		async move {
			let response = create_api_token(access_token.clone(), request.clone()).await;

			if let Ok(_) = response {
				api_tokens_list_query.invalidate_query(AllApiTokensTag);
			}

			response
		}
	})
}
