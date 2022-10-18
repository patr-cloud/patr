use std::collections::HashMap;

use api_models::utils::Uuid;
use chrono::{DateTime, Utc};
use eve_rs::AsError;
use sqlx::types::ipnetwork::IpNetwork;

use crate::{models::rbac::WorkspacePermissions, utils::Error, error};

#[derive(Clone, Debug)]
// Sample Token: patrv1.{token}.loginId
pub struct UserApiTokenData {
	pub token_id: Uuid,
	pub user_id: Uuid,
	pub token_nbf: Option<DateTime<Utc>>,
	pub token_exp: Option<DateTime<Utc>>,
	pub allowed_ips: Option<Vec<IpNetwork>>,
	pub created: DateTime<Utc>,
	pub revoked: Option<DateTime<Utc>>,
	pub workspaces: HashMap<Uuid, WorkspacePermissions>,
}

impl UserApiTokenData {
	pub fn parse(token: &str) -> Result<Self, Error> {
		// Split by dots, then parse accordingly.
		let mut splitter = token.split('.');
		let version = splitter.next().status(400).body(error!(UNAUTHORIZED).to_string())?;
		if version != "patrv1" {
			return Err(Error::empty().status(400).body(error!(UNAUTHORIZED).to_string()));
		}
		let token = splitter.next().status(400).body(error!(UNAUTHORIZED).to_string())?;
		let login_id = splitter.next().status(400).body(error!(UNAUTHORIZED).to_string())?;
		let login_id = Uuid::parse_str(&login_id)?;
	}
}
