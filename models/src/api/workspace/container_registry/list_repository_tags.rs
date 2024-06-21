use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::prelude::*;

/// The response body for the ListContainerRepositories endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContainerRepositoryTagAndDigestInfo {
	/// The tag of the repository
	pub tag: String,
	/// The digest that this tag points to
	pub digest: String,
	/// The last updated time of the tag
	pub last_updated: OffsetDateTime,
}

macros::declare_api_endpoint!(
	/// Route to get list of all container repositories in a workspace
	ListContainerRepositoryTags,
	GET "/workspace/:workspace_id/container-registry/:repository_id/tag" {
		/// The workspace ID to list the container registry repositories in
		pub workspace_id: Uuid,
		/// The container repository ID to list the tags of
		pub repository_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.repository_id,
			permission: Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::View),
		}
	},
	pagination = true,
	response_headers = {
		/// The total number of container repositories in the requested workspace
		pub total_count: TotalCountHeader,
	},
	response = {
		/// List of tags in the current container repository
		pub tags: Vec<ContainerRepositoryTagAndDigestInfo>
	}
);
