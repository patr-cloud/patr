use std::net::IpAddr;

use api_models::utils::Uuid;
use chrono::Utc;
use serde_json::{json, Value};

use crate::{db, Database};

/// AuditLogData used for logging actions/events to DB
#[derive(Debug)]
pub enum AuditLogData {
	/// Data used to add audit logs for Workspace routes
	WorkspaceResource {
		/// The workspace for which the audit log has to be added,
		/// refers to workspace table's id
		workspace_id: Uuid,
		/// The resource id on which the action is done,
		/// refers to resourse table's id
		resource_id: Uuid,
		/// The action which is done on the resource,
		/// refers to permission table's id
		action_id: Uuid,
		/// Optional action specific metadata that needs to be logged
		metadata: Option<Value>,
	},
}

pub async fn add_audit_log(
	connection: &mut <Database as sqlx::Database>::Connection,
	request_id: &Uuid,
	ip_addr: &IpAddr,
	user_id: &Uuid,
	login_id: &Uuid,
	audit_log_data: AuditLogData,
) -> Result<(), sqlx::Error> {
	match audit_log_data {
		AuditLogData::WorkspaceResource {
			workspace_id,
			resource_id,
			action_id,
			metadata,
		} => {
			let audit_log_id =
				db::generate_new_workspace_audit_log_id(connection).await?;

			db::create_workspace_audit_log(
				connection,
				&audit_log_id,
				&workspace_id,
				&ip_addr.to_string(),
				Utc::now().into(),
				Some(user_id),
				Some(login_id),
				&resource_id,
				&action_id,
				request_id,
				&metadata.unwrap_or_else(|| json!({})),
				false, // action done by the user thorough api
				true,
			)
			.await?;
		}
	};
	Ok(())
}
