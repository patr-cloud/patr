use std::collections::BTreeMap;

use crate::{prelude::*, api::workspace::infrastructure::deployment::ExposedPortType};

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
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id
		}
	},
	response = {
		/// The exposed ports of the container repository.
		pub ports: BTreeMap<StringifiedU16, ExposedPortType>,
	}
);
