use super::DnsRecordValue;
use crate::{prelude::*, utils::constants::DNS_RECORD_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Route to add new DNS record
	AddDNSRecord,
	POST "/workspace/:workspace_id/domain/:domain_id/dns-record" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The ID of the domain to add the DNS record for
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
			permission: Permission::DnsRecord(DnsRecordPermission::Add),
		}
	},
	request = {
		/// The name of the new record
		#[preprocess(trim, lowercase, regex = DNS_RECORD_NAME_REGEX)]
		pub name: String,
		/// The type of the new record
		/// It can be of type:
		///     A
		///     MX
		///     TXT
		///     AAAA
		///     CNAME
		#[serde(flatten)]
		#[preprocess(none)]
		pub r#type: DnsRecordValue,
		/// The time to live of a record
		// #[preprocess(type = "u32")]
		pub ttl: u32,
	},
	response = {
		/// The ID of the created record
		#[serde(flatten)]
		pub id: WithId<()>,
	}
);
