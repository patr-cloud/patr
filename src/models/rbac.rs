use std::collections::HashMap;

use once_cell::sync::OnceCell;
use uuid::Uuid;

static GOD_USER_ID: OnceCell<Uuid> = OnceCell::new();

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrgPermissions {
	pub is_super_admin: bool,
	pub resources: HashMap<Vec<u8>, Vec<String>>, /* Given a resource, what and all permissions do you have on it */
	pub resource_types: HashMap<String, Vec<String>>, /* Given a resource type, what and all permissions do you have on it */
}

#[allow(dead_code)]
pub mod permissions {
	pub mod docker {
		pub const PUSH: &str = "docker::push";
		pub const PULL: &str = "docker::pull";
	}

	pub mod deployer {
		pub const DEPLOY: &str = "deployer::deploy";
	}
}
