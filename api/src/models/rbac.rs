use std::collections::HashMap;

use once_cell::sync::OnceCell;
use serde_derive::{Deserialize, Serialize};
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
	pub mod organisation {
		pub mod domain {
			pub const LIST: &str = "organisation::domain::list";
			pub const ADD: &str = "organisation::domain::add";
			pub const VIEW_DETAILS: &str = "organisation::domain::viewDetails";
			pub const VERIFY: &str = "organisation::domain::verify";
			pub const DELETE: &str = "organisation::domain::delete";
		}

		pub mod application {
			pub const LIST: &str = "organisation::application::list";
			pub const ADD: &str = "organisation::application::add";
			pub const VIEW_DETAILS: &str =
				"organisation::application::viewDetails";
			pub const DELETE: &str = "organisation::application::delete";
			pub const LIST_VERSIONS: &str =
				"organisation::application::listVersions";
		}

		pub const VIEW_ROLES: &str = "organisation::viewRoles";
		pub const CREATE_ROLE: &str = "organisation::createRole";
		pub const EDIT_ROLE: &str = "organisation::editRole";
		pub const DELETE_ROLE: &str = "organisation::deleteRole";
		pub const EDIT_INFO: &str = "organisation::editInfo";
	}

	pub mod docker {
		pub const PUSH: &str = "docker::push";
		pub const PULL: &str = "docker::pull";
	}

	pub mod deployer {
		pub const DEPLOY: &str = "deployer::deploy";
	}
}

#[allow(dead_code)]
#[api_macros::iterable_module(consts, recursive = false)]
pub mod resource_types {
	pub const ORGANISATION: &str = "organisation";
	pub const DOMAIN: &str = "domain";
	pub const APPLICATION: &str = "application";
}
