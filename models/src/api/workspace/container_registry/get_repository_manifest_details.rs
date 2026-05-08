use super::ContainerRepositoryManifestInfo;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Gets the details of a container repository's manifest in the workspace.
	GetContainerRepositoryManifestDetails,
	GET "/workspace/{workspace_id}/container-registry/{repository_id}/manifest/{digest_or_tag}" {
		/// The workspace to get the container repository in.
		pub workspace_id: Uuid,
		/// The id of the repository to get the manifest details of.
		pub repository_id: Uuid,
		/// The digest of the manifest to get the details of.
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
			extract_resource_id: |req| req.path.repository_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::View),
		}
	},
	response = {
		/// The details of the container repository's manifest.
		#[serde(flatten)]
		pub manifest_details: ContainerRepositoryManifestInfo,
		/// The sub-manifests referenced by this manifest, if it's an index manifest. This field will be empty for image manifests.
		#[serde(default, skip_serializing_if = "Vec::is_empty")]
		pub referenced_manifests: Vec<ContainerRepositoryManifestInfo>,
	},
	audit_log = NoAuditLogger,
);
