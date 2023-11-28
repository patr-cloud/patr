use crate::{prelude::*, utils::BearerToken};
use std::{fmt::Display, str::FromStr};
use serde::{Serialize, Deserialize};

#[cfg(feature = "server")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "camelCase")]
#[sqlx(type_name = "DOMAIN_NAMESERVER_TYPE", rename_all = "lowercase")]
pub enum DomainNameserverType {
	Internal,
	External,
}

/// Type of domain nameserver
#[cfg(not(feature = "server"))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DomainNameserverType {
	/// Internal
	Internal,
	/// External
	External,
}

impl DomainNameserverType {
	/// To check if external
	pub fn is_external(&self) -> bool {
		matches!(self, DomainNameserverType::External)
	}

	/// To check if internal
	pub fn is_internal(&self) -> bool {
		matches!(self, DomainNameserverType::Internal)
	}
}

impl Display for DomainNameserverType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Internal => write!(f, "internal"),
			Self::External => write!(f, "external"),
		}
	}
}

impl FromStr for DomainNameserverType {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = s.to_lowercase();
		match s.as_str() {
			"internal" => Ok(Self::Internal),
			"external" => Ok(Self::External),
			_ => Err(s),
		}
	}
}

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
			extract_resource_id: |req| req.path.workspace_id
		}
	},
	request = {
		/// The name of the domain
		pub domain: String,
		/// The type of nameserver
		/// It can be
		///     Internal
		///     External
		pub nameserver_type: DomainNameserverType,
	},
	response = {
		/// The ID of the created record
		#[serde(flatten)]
		pub id: WithId<()>
	}
);
