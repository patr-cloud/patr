use std::collections::HashMap;

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub static GOD_USER_ID: OnceCell<Uuid> = OnceCell::new();
pub static RESOURCE_TYPES: OnceCell<HashMap<String, Vec<u8>>> = OnceCell::new();

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OrgPermissions {
	pub is_super_admin: bool,
	pub resources: HashMap<Vec<u8>, Vec<String>>, /* Given a resource, what
	                                               * and all permissions do
	                                               * you have on it */
	pub resource_types: HashMap<Vec<u8>, Vec<String>>, /* Given a resource
	                                                    * type, what and all
	                                                    * permissions do you
	                                                    * have on it */
}

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

		#[allow(dead_code)]
		pub mod application {
			pub const LIST: &str = "organisation::application::list";
			pub const ADD: &str = "organisation::application::add";
			pub const VIEW_DETAILS: &str =
				"organisation::application::viewDetails";
			pub const DELETE: &str = "organisation::application::delete";
			pub const LIST_VERSIONS: &str =
				"organisation::application::listVersions";
		}

		pub mod portus {
			pub const ADD: &str = "organisation::portus::add";
			pub const VIEW: &str = "organisation::portus::view";
			pub const LIST: &str = "organisation::portus::list";
			pub const DELETE: &str = "organisation::portus::delete";
		}

		pub mod deployment {
			pub const LIST: &str = "organisation::deployment::list";
			pub const CREATE: &str = "organisation::deployment::create";
			pub const INFO: &str = "organisation::deployment::info";
			pub const DELETE: &str = "organisation::deployment::delete";
			pub const EDIT: &str = "organisation::deployment::edit";

			#[allow(dead_code)]
			pub mod upgrade_path {
				pub const LIST: &str =
					"organisation::deployment::upgradePath::list";
				pub const CREATE: &str =
					"organisation::deployment::upgradePath::create";
				pub const INFO: &str =
					"organisation::deployment::upgradePath::info";
				pub const DELETE: &str =
					"organisation::deployment::upgradePath::delete";
				pub const EDIT: &str =
					"organisation::deployment::upgradePath::edit";
			}

			#[allow(dead_code)]
			pub mod entry_point {
				pub const LIST: &str =
					"organisation::deployment::entryPoint::list";
				pub const CREATE: &str =
					"organisation::deployment::entryPoint::create";
				pub const EDIT: &str =
					"organisation::deployment::entryPoint::edit";
				pub const DELETE: &str =
					"organisation::deployment::entryPoint::delete";
			}
		}

		pub mod docker_registry {
			pub const CREATE: &str = "organisation::dockerRegistry::create";
			pub const LIST: &str = "organisation::dockerRegistry::list";
			pub const DELETE: &str = "organisation::dockerRegistry::delete";
			pub const PUSH: &str = "organisation::dockerRegistry::push";
			pub const PULL: &str = "organisation::dockerRegistry::pull";
		}

		pub mod managed_database {
			pub const CREATE: &str = "organisation::managedDatabase::create";
			pub const LIST: &str = "organisation::managedDatabase::list";
			pub const DELETE: &str = "organisation::managedDatabase::delete";
			pub const INFO: &str = "organisation::managedDatabase::info";
		}

		pub mod static_site {
			pub const LIST: &str = "organisation::staticSite::list";
			pub const CREATE: &str = "organisation::staticSite::create";
			pub const INFO: &str = "organisation::staticSite::info";
			pub const DELETE: &str = "organisation::staticSite::delete";
			pub const EDIT: &str = "organisation::staticSite::edit";
		}

		pub const VIEW_ROLES: &str = "organisation::viewRoles";
		pub const CREATE_ROLE: &str = "organisation::createRole";
		pub const EDIT_ROLE: &str = "organisation::editRole";
		pub const DELETE_ROLE: &str = "organisation::deleteRole";
		#[allow(dead_code)]
		pub const EDIT_INFO: &str = "organisation::editInfo";
	}
}

#[allow(dead_code)]
#[api_macros::iterable_module(consts, recursive = false)]
pub mod resource_types {
	pub const ORGANISATION: &str = "organisation";
	pub const DOMAIN: &str = "domain";
	pub const APPLICATION: &str = "application";
	pub const PORTUS: &str = "portus";
	pub const DOCKER_REPOSITORY: &str = "dockerRepository";
	pub const MANAGED_DATABASE: &str = "managedDatabase";
	pub const DEPLOYMENT: &str = "deployment";
	pub const STATIC_SITE: &str = "staticSite";
	pub const DEPLOYMENT_UPGRADE_PATH: &str = "deploymentUpgradePath";
	pub const DEPLOYMENT_ENTRY_POINT: &str = "deploymentEntryPoint";
}
