use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to delete a DNS record
	DeleteDNSRecord,
	DELETE "/workspace/:workspace_id/domain/:domain_id/dns-record/:record_id" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The domain ID of the record
		pub domain_id: Uuid,
		/// The record ID to be deleted
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
			permission: Permission::DnsRecord(DnsRecordPermission::Delete),
		}
	}
);
