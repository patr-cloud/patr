use std::collections::HashMap;

use once_cell::sync::OnceCell;
use uuid::Uuid;

pub static GOD_USER_ID: OnceCell<Uuid> = OnceCell::new();
pub static RESOURCE_TYPES: OnceCell<HashMap<String, Vec<u8>>> = OnceCell::new();

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OrgPermissions {
	pub is_super_admin: bool,
	pub resources: HashMap<Vec<u8>, Vec<String>>, /* Given a resource, what and all permissions do you have on it */
	pub resource_types: HashMap<Vec<u8>, Vec<String>>, /* Given a resource type, what and all permissions do you have on it */
}

#[allow(dead_code)]
#[api_macros::iterable_module(consts, recursive = true)]
pub mod permissions {
	pub mod docker {
		pub const PUSH: &str = "docker::push";
		pub const PULL: &str = "docker::pull";
	}

	pub mod deployer {
		pub const DEPLOY: &str = "deployer::deploy";
	}

	pub mod organisation {
		pub mod domain {
			pub const VIEW: &str = "organisation::domain::view";
			pub const ADD: &str = "organisation::domain::add";
			pub const VERIFY: &str = "organisation::domain::verify";
		}
	}
}

#[allow(dead_code)]
#[api_macros::iterable_module(consts, recursive = false)]
pub mod resource_types {
	pub const ORGANISATION: &str = "organisation";
	pub const DOMAIN: &str = "domain";
}
