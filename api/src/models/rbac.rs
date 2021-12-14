use std::collections::HashMap;

use api_models::utils::Uuid;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

pub static GOD_USER_ID: OnceCell<Uuid> = OnceCell::new();
// A mapping of resource type name -> resource type IDs
pub static RESOURCE_TYPES: OnceCell<HashMap<String, Uuid>> = OnceCell::new();
// A mapping of permission names -> permission IDs
pub static PERMISSIONS: OnceCell<HashMap<String, Uuid>> = OnceCell::new();

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePermissions {
	pub is_super_admin: bool,
	pub resources: HashMap<Uuid, Vec<Uuid>>, /* Given a resource, what
	                                          * and all permissions do
	                                          * you have on it */
	pub resource_types: HashMap<Uuid, Vec<Uuid>>, /* Given a resource
	                                               * type, what and all
	                                               * permissions do you
	                                               * have on it */
}

#[api_macros::iterable_module(consts, recursive = true)]
pub mod permissions {
	pub mod workspace {
		pub mod domain {
			pub const LIST: &str = "workspace::domain::list";
			pub const ADD: &str = "workspace::domain::add";
			pub const VIEW_DETAILS: &str = "workspace::domain::viewDetails";
			pub const VERIFY: &str = "workspace::domain::verify";
			pub const DELETE: &str = "workspace::domain::delete";
		}

		pub mod deployment {
			pub const LIST: &str = "workspace::deployment::list";
			pub const CREATE: &str = "workspace::deployment::create";
			pub const INFO: &str = "workspace::deployment::info";
			pub const DELETE: &str = "workspace::deployment::delete";
			pub const EDIT: &str = "workspace::deployment::edit";

			#[allow(dead_code)]
			pub mod upgrade_path {
				pub const LIST: &str =
					"workspace::deployment::upgradePath::list";
				pub const CREATE: &str =
					"workspace::deployment::upgradePath::create";
				pub const INFO: &str =
					"workspace::deployment::upgradePath::info";
				pub const DELETE: &str =
					"workspace::deployment::upgradePath::delete";
				pub const EDIT: &str =
					"workspace::deployment::upgradePath::edit";
			}

			#[allow(dead_code)]
			pub mod entry_point {
				pub const LIST: &str =
					"workspace::deployment::entryPoint::list";
				pub const CREATE: &str =
					"workspace::deployment::entryPoint::create";
				pub const EDIT: &str =
					"workspace::deployment::entryPoint::edit";
				pub const DELETE: &str =
					"workspace::deployment::entryPoint::delete";
			}
		}

		pub mod docker_registry {
			pub const CREATE: &str = "workspace::dockerRegistry::create";
			pub const LIST: &str = "workspace::dockerRegistry::list";
			pub const DELETE: &str = "workspace::dockerRegistry::delete";
			pub const PUSH: &str = "workspace::dockerRegistry::push";
			pub const PULL: &str = "workspace::dockerRegistry::pull";
		}

		pub mod managed_database {
			pub const CREATE: &str = "workspace::managedDatabase::create";
			pub const LIST: &str = "workspace::managedDatabase::list";
			pub const DELETE: &str = "workspace::managedDatabase::delete";
			pub const INFO: &str = "workspace::managedDatabase::info";
		}

		pub mod static_site {
			pub const LIST: &str = "workspace::staticSite::list";
			pub const CREATE: &str = "workspace::staticSite::create";
			pub const INFO: &str = "workspace::staticSite::info";
			pub const DELETE: &str = "workspace::staticSite::delete";
			pub const EDIT: &str = "workspace::staticSite::edit";
		}

		pub const VIEW_ROLES: &str = "workspace::viewRoles";
		pub const CREATE_ROLE: &str = "workspace::createRole";
		pub const EDIT_ROLE: &str = "workspace::editRole";
		pub const DELETE_ROLE: &str = "workspace::deleteRole";
		#[allow(dead_code)]
		pub const EDIT_INFO: &str = "workspace::editInfo";
	}
}

#[allow(dead_code)]
#[api_macros::iterable_module(consts, recursive = false)]
pub mod resource_types {
	pub const WORKSPACE: &str = "workspace";
	pub const DOMAIN: &str = "domain";
	pub const DOCKER_REPOSITORY: &str = "dockerRepository";
	pub const MANAGED_DATABASE: &str = "managedDatabase";
	pub const DEPLOYMENT: &str = "deployment";
	pub const STATIC_SITE: &str = "staticSite";
	pub const DEPLOYMENT_UPGRADE_PATH: &str = "deploymentUpgradePath";
	pub const DEPLOYMENT_ENTRY_POINT: &str = "deploymentEntryPoint";
}
