use std::collections::{BTreeMap, BTreeSet};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod billing;
pub mod ci;
pub mod docker_registry;
pub mod domain;
pub mod infrastructure;
pub mod rbac;
pub mod region;
pub mod secret;

mod create_workspace;
mod delete_workspace;
mod get_resource_audit_log;
mod get_workspace_audit_log;
mod get_workspace_info;
mod is_name_available;
mod update_workspace_info;

pub use self::{
	create_workspace::*,
	delete_workspace::*,
	get_resource_audit_log::*,
	get_workspace_audit_log::*,
	get_workspace_info::*,
	is_name_available::*,
	update_workspace_info::*,
};
use crate::utils::{DateTime, Uuid};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
	pub id: Uuid,
	pub name: String,
	pub super_admin_id: Uuid,
	pub active: bool,
	pub alert_emails: Vec<String>,
	pub default_payment_method_id: Option<String>,
	pub is_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceAuditLog {
	pub id: Uuid,
	pub date: DateTime<Utc>,
	pub ip_address: String,
	pub workspace_id: Uuid,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub user_id: Option<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub login_id: Option<Uuid>,
	pub resource_id: Uuid,
	pub action: String,
	pub request_id: Uuid,
	pub metadata: Value,
	pub patr_action: bool,
	pub request_success: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePermission {
	pub is_super_admin: bool,
	pub resource_permissions: BTreeMap<Uuid, BTreeSet<Uuid>>,
	pub resource_type_permissions: BTreeMap<Uuid, BTreeSet<Uuid>>,
}

#[cfg(test)]
mod test {
	use std::collections::{BTreeMap, BTreeSet};

	use serde_test::{assert_tokens, Token};

	use super::{Workspace, WorkspacePermission};
	use crate::utils::Uuid;

	#[test]
	fn assert_workspace_types() {
		assert_tokens(
			&Workspace {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				name: "John Patr's Company".to_string(),
				super_admin_id: Uuid::parse_str(
					"2aef18631ded45eb9170dc2166b30898",
				)
				.unwrap(),
				active: true,
				alert_emails: vec!["johnpatr@patr.com".to_string()],
				default_payment_method_id: Some(
					"pm_6K95KhSGEPBh7GrIsWVB4pyV".to_string(),
				),
				is_verified: true,
			},
			&[
				Token::Struct {
					name: "Workspace",
					len: 7,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("John Patr's Company"),
				Token::Str("superAdminId"),
				Token::Str("2aef18631ded45eb9170dc2166b30898"),
				Token::Str("active"),
				Token::Bool(true),
				Token::Str("alertEmails"),
				Token::Seq { len: Some(1) },
				Token::Str("johnpatr@patr.com"),
				Token::SeqEnd,
				Token::Str("defaultPaymentMethodId"),
				Token::Some,
				Token::Str("pm_6K95KhSGEPBh7GrIsWVB4pyV"),
				Token::Str("isVerified"),
				Token::Bool(true),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_workspace_permission_types() {
		assert_tokens(
			&WorkspacePermission {
				is_super_admin: true,
				resource_permissions: {
					let mut map = BTreeMap::new();

					map.insert(
						Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						BTreeSet::from([
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
								.unwrap(),
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30869")
								.unwrap(),
						]),
					);

					map
				},
				resource_type_permissions: {
					let mut map = BTreeMap::new();

					map.insert(
						Uuid::parse_str("2aef18631ded45eb9170dc2166b30877")
							.unwrap(),
						BTreeSet::from([
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30878")
								.unwrap(),
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30879")
								.unwrap(),
						]),
					);

					map
				},
			},
			&[
				Token::Struct {
					name: "WorkspacePermission",
					len: 3,
				},
				Token::Str("isSuperAdmin"),
				Token::Bool(true),
				Token::Str("resourcePermissions"),
				Token::Map { len: Some(1) },
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Seq { len: Some(2) },
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("2aef18631ded45eb9170dc2166b30869"),
				Token::SeqEnd,
				Token::MapEnd,
				Token::Str("resourceTypePermissions"),
				Token::Map { len: Some(1) },
				Token::Str("2aef18631ded45eb9170dc2166b30877"),
				Token::Seq { len: Some(2) },
				Token::Str("2aef18631ded45eb9170dc2166b30878"),
				Token::Str("2aef18631ded45eb9170dc2166b30879"),
				Token::SeqEnd,
				Token::MapEnd,
				Token::StructEnd,
			],
		)
	}
}
