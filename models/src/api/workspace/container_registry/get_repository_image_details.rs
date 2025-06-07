use time::OffsetDateTime;

use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Gets the details of a container repository's image in the workspace.
	GetContainerRepositoryImageDetails,
	GET "/workspace/:workspace_id/container-registry/:repository_id/image/:digest_or_tag" {
		/// The workspace to get the container repository in.
		pub workspace_id: Uuid,
		/// The id of the repository to get the image details of.
		pub repository_id: Uuid,
		/// The digest of the image to get the details of.
		pub digest_or_tag: String,
	},
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id,
			permission: Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::View),
		}
	},
	response = {
		/// The digest of the container repository's image.
		pub digest: String,
		/// The size of the container repository's image.
		pub size: u64,
		/// The creation date of the container repository's image.
		///
		/// TODO: Change this to audit log
		pub created: OffsetDateTime,
		/// The tags of the container repository's image.
		pub tags: Vec<String>,
	}
);
