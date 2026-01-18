use serde::{Deserialize, Serialize};

use super::MaybeExternallySourced;
use crate::{api::workspace::domain::DomainNameserverType, prelude::*};

/// The IaaC domain resource. This is used to define a domain in an IaaC file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct IaacDomain {
	/// The ID of the domain. This is optional and is used to update an existing
	/// domain. If not provided, a new domain will be created.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub id: Option<Uuid>,
	/// The name of the domain
	pub name: MaybeExternallySourced<String>,
	/// The type of nameserver for the domain. Can be Internal or External.
	/// - Internal: The nameserver is managed by Patr
	/// - External: The nameserver is managed by the user
	pub nameserver_type: MaybeExternallySourced<DomainNameserverType>,
}
