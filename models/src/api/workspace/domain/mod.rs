use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::prelude::*;

/// The endpoint to add a domain to a workspace
mod add_domain_to_workspace;
/// The endpoint to delete a domain from a workspace
mod delete_domain_in_workspace;
/// The endpoint to get the domain information in a workspace
mod get_domain_info_in_workspace;
/// The endpoint to check if a domain is valid and can be added to a workspace
mod is_domain_valid;
/// The endpoint to get all the domains in a workspace
mod list_domains_in_workspace;
/// The endpoint to verify a domain in a workspace
mod verify_domain_in_workspace;

pub use self::{
	add_domain_to_workspace::*,
	delete_domain_in_workspace::*,
	get_domain_info_in_workspace::*,
	is_domain_valid::*,
	list_domains_in_workspace::*,
	verify_domain_in_workspace::*,
};

/// The domain information in a workspace
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ListableResource, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDomain {
	/// The name of the domain
	pub name: String,
	/// Last verified time of the domain
	#[ts(type = "Date")]
	pub last_verified: Option<OffsetDateTime>,
	/// Whether or not the domain is verified
	pub is_verified: bool,
}
