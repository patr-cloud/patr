use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::GitProvider;
use crate::{
	utils::{Paginated, Uuid},
	ApiRequest,
};

#[derive(
	Eq,
	Ord,
	Hash,
	Debug,
	Clone,
	Default,
	TypedPath,
	PartialEq,
	Serialize,
	PartialOrd,
	Deserialize,
)]
#[typed_path("/workspace/:workspace_id/ci/git-provider")]
pub struct ListGitProvidersPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListGitProvidersRequest {}

impl ApiRequest for ListGitProvidersRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListGitProvidersPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListGitProvidersResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListGitProvidersResponse {
	pub git_providers: Vec<GitProvider>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{
		GitProvider,
		ListGitProvidersRequest,
		ListGitProvidersResponse,
	};
	use crate::{
		models::workspace::ci::git_provider::GitProviderType,
		utils::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListGitProvidersRequest {},
			&[
				Token::Struct {
					name: "ListGitProvidersRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListGitProvidersResponse {
				git_providers: vec![GitProvider {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					domain_name: "github.com".to_string(),
					git_provider_type: GitProviderType::Github,
					login_name: Some("login-name".to_string()),
					is_syncing: false,
					last_synced: None,
					is_deleted: false,
				}],
			},
			&[
				Token::Struct {
					name: "ListGitProvidersResponse",
					len: 1,
				},
				Token::Str("gitProviders"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "GitProvider",
					len: 7,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("domainName"),
				Token::Str("github.com"),
				Token::Str("gitProviderType"),
				Token::Enum {
					name: "GitProviderType",
				},
				Token::Str("github"),
				Token::Unit,
				Token::Str("loginName"),
				Token::Some,
				Token::Str("login-name"),
				Token::Str("isSyncing"),
				Token::Bool(false),
				Token::Str("lastSynced"),
				Token::None,
				Token::Str("isDeleted"),
				Token::Bool(false),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListGitProvidersResponse {
				git_providers: vec![GitProvider {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					domain_name: "github.com".to_string(),
					git_provider_type: GitProviderType::Github,
					login_name: Some("login-name".to_string()),
					is_syncing: false,
					last_synced: None,
					is_deleted: false,
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("gitProviders"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "GitProvider",
					len: 7,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("domainName"),
				Token::Str("github.com"),
				Token::Str("gitProviderType"),
				Token::Enum {
					name: "GitProviderType",
				},
				Token::Str("github"),
				Token::Unit,
				Token::Str("loginName"),
				Token::Some,
				Token::Str("login-name"),
				Token::Str("isSyncing"),
				Token::Bool(false),
				Token::Str("lastSynced"),
				Token::None,
				Token::Str("isDeleted"),
				Token::Bool(false),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		)
	}
}
