use std::collections::BTreeMap;

use crate::{api::workspace::deployment::ExposedPortType, prelude::*};

macros::declare_api_endpoint!(
	/// Gets the exposed ports of a container repository in the workspace.
	GetContainerRepositoryExposedPorts,
	GET "/workspace/:workspace_id/container-registry/:repository_id/image/:digest_or_tag/exposed-ports" {
		/// The workspace to get the container repository in.
		pub workspace_id: Uuid,
		/// The id of the repository to get the exposed ports of.
		pub repository_id: Uuid,
		/// The digest of the image to get the exposed ports of.
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
		/// The exposed ports of the container repository.
		pub ports: BTreeMap<StringifiedU16, ExposedPortType>,
	}
);
