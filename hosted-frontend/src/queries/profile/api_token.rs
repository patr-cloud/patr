use leptos_query::*;
use models::api::user::*;

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
