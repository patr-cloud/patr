use super::WorkspaceDomain;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to get domain information
	GetDomainInfoInWorkspace,
	GET "/workspace/:workspace_id/domain/:domain_id" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The domain ID
		pub domain_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.domain_id,
			permission: Permission::Domain(DomainPermission::View),
		}
	},
	response = {
		/// The domain information containing:
		/// - domain - The domain metadata
		/// - is_verified - whether the domain is verified or not
		/// - nameserver_type - The type of the nameserver
		#[serde(flatten)]
		pub workspace_domain: WithId<WorkspaceDomain>,
	}
);
