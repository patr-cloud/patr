use super::DomainNameserverType;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to add domain to a workspace
	AddDomainToWorkspace,
	POST "/workspace/:workspace_id/domain" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id,
			permission: Permission::Domain(DomainPermission::Add),
		}
	},
	request = {
		/// The name of the domain
		#[preprocess(domain)]
		pub domain: String,
		/// The type of nameserver
		/// It can be
		/// - Internal: The nameserver is managed by Patr
		/// - External: The nameserver is managed by the user
		#[preprocess(none)]
		pub nameserver_type: DomainNameserverType,
	},
	response = {
		/// The ID of the created record
		#[serde(flatten)]
		pub id: WithId<()>,
	}
);
