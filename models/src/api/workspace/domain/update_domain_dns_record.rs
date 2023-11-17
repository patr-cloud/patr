use crate::{prelude::*, utils::BearerToken};

macros::declare_api_endpoint!(
	/// Route to update the domains DNS record
	UpdateDomainDNSRecord,
	PATCH "/workspace/:workspace_id/domain/:domain_id/dns-record/:record_id" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The domain ID of the record
		pub domain_id: Uuid,
		/// The record ID to be updated
		pub record_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.domain_id
		}
	},
	request = {
		/// To update the time to live of the domain
		pub ttl: Option<u32>,
		/// To update the tagert of the domain
		pub target: Option<String>,
		/// To update the priority of the domain
		pub priority: Option<u16>,
		/// To update the if the domain is proxied or not
		pub proxied: Option<bool>,
	},
	response = {
		/// The ID of the created record
		#[serde(flatten)]
		pub id: WithId<()>
	}
);
