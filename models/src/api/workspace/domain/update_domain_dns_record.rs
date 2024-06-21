use crate::prelude::*;

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
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.record_id,
			permission: Permission::DnsRecord(DnsRecordPermission::Edit),
		}
	},
	request = {
		/// To update the time to live of the domain
		// #[preprocess(type = "u32")]
		pub ttl: Option<u32>,
		/// To update the target of the domain
		// #[preprocess(trim, lowercase)]
		pub target: Option<String>,
		/// To update the priority of the domain
		// #[preprocess(type = "u16")]
		pub priority: Option<u16>,
		/// To update the if the domain is proxied or not
		// #[preprocess(none)]
		pub proxied: Option<bool>,
	}
);
