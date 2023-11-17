use crate::{prelude::*, utils::BearerToken};
use serde::{Serialize, Deserialize};
use super::DnsRecordValue;

/// The DNS record information of patr domain
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PatrDomainDnsRecord {
	/// The domain ID
	pub domain_id: Uuid,
	/// The domain name
	pub name: String,
	/// The domain type
	#[serde(flatten)]
	pub r#type: DnsRecordValue,
	/// The time to live
	pub ttl: u32,
}

macros::declare_api_endpoint!(
	/// Route to get domain DNS record
	GetDomainDNSrecord,
	GET "/workspace/:workspace_id/domain/:domain_id/dns-record" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The domain ID
		pub domain_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id
		}
	},
	response = {
		/// The list of records containing:
		///     domain_id -  The domain ID
		///     name - The domain name
		///     r#type - The domain type
		///     ttl - The time to live
		pub records: Vec<WithId<PatrDomainDnsRecord>>,
	}
);
