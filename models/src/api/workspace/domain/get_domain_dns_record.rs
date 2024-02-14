use super::PatrDomainDnsRecord;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to get domain DNS record
	GetDomainDNSRecord,
	GET "/workspace/:workspace_id/domain/:domain_id/dns-record" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The domain ID
		pub domain_id: Uuid,
	},
	pagination = true,
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.domain_id
		}
	},
	response_headers = {
		/// The total number of items in the pagination
		pub total_count: TotalCountHeader,
	},
	response = {
		/// The list of records containing:
		/// - domain_id - The domain ID
		/// - name - The domain name
		/// - type - The domain type
		/// - ttl - The time to live
		pub records: Vec<WithId<PatrDomainDnsRecord>>,
	}
);
