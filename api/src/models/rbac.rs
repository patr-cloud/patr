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

			pub mod dns_record {
				pub const LIST: &str = "workspace::domain::dnsRecord::list";
				pub const ADD: &str = "workspace::domain::dnsRecord::add";
				pub const EDIT: &str = "workspace::domain::dnsRecord::edit";
				pub const DELETE: &str = "workspace::domain::dnsRecord::delete";
			}
		}

		pub mod infrastructure {
			pub mod deployment {
				pub const LIST: &str =
					"workspace::infrastructure::deployment::list";
				pub const CREATE: &str =
					"workspace::infrastructure::deployment::create";
				pub const INFO: &str =
					"workspace::infrastructure::deployment::info";
				pub const DELETE: &str =
					"workspace::infrastructure::deployment::delete";
				pub const EDIT: &str =
					"workspace::infrastructure::deployment::edit";
			}

			#[allow(dead_code)]
			pub mod upgrade_path {
				pub const LIST: &str =
					"workspace::infrastructure::upgradePath::list";
				pub const CREATE: &str =
					"workspace::infrastructure::upgradePath::create";
				pub const INFO: &str =
					"workspace::infrastructure::upgradePath::info";
				pub const DELETE: &str =
					"workspace::infrastructure::upgradePath::delete";
				pub const EDIT: &str =
					"workspace::infrastructure::upgradePath::edit";
			}

			pub mod managed_url {
				pub const LIST: &str =
					"workspace::infrastructure::managedUrl::list";
				pub const CREATE: &str =
					"workspace::infrastructure::managedUrl::create";
				pub const EDIT: &str =
					"workspace::infrastructure::managedUrl::edit";
				pub const DELETE: &str =
					"workspace::infrastructure::managedUrl::delete";
			}

			pub mod managed_database {
				pub const CREATE: &str =
					"workspace::infrastructure::managedDatabase::create";
				pub const LIST: &str =
					"workspace::infrastructure::managedDatabase::list";
				pub const DELETE: &str =
					"workspace::infrastructure::managedDatabase::delete";
				pub const INFO: &str =
					"workspace::infrastructure::managedDatabase::info";
			}

			pub mod static_site {
				pub const LIST: &str =
					"workspace::infrastructure::staticSite::list";
				pub const CREATE: &str =
					"workspace::infrastructure::staticSite::create";
				pub const INFO: &str =
					"workspace::infrastructure::staticSite::info";
				pub const DELETE: &str =
					"workspace::infrastructure::staticSite::delete";
				pub const EDIT: &str =
					"workspace::infrastructure::staticSite::edit";
			}
		}

		pub mod docker_registry {
			pub const CREATE: &str = "workspace::dockerRegistry::create";
			pub const LIST: &str = "workspace::dockerRegistry::list";
			pub const DELETE: &str = "workspace::dockerRegistry::delete";
			pub const INFO: &str = "workspace::dockerRegistry::info";
			pub const PUSH: &str = "workspace::dockerRegistry::push";
			pub const PULL: &str = "workspace::dockerRegistry::pull";
		}

		pub mod secret {
			pub const LIST: &str = "workspace::secret::list";
			pub const CREATE: &str = "workspace::secret::create";
			pub const EDIT: &str = "workspace::secret::edit";
			pub const DELETE: &str = "workspace::secret::delete";
		}

		pub mod rbac {
			pub mod roles {
				pub const LIST: &str = "workspace::rbac::role::list";
				pub const CREATE: &str = "workspace::rbac::role::create";
				pub const EDIT: &str = "workspace::rbac::role::edit";
				pub const DELETE: &str = "workspace::rbac::role::delete";
			}

			pub mod user {
				pub const LIST: &str = "workspace::rbac::user::list";
				pub const ADD: &str = "workspace::rbac::user::add";
				pub const REMOVE: &str = "workspace::rbac::user::remove";
				pub const UPDATE_ROLES: &str =
					"workspace::rbac::user::updateRoles";
			}
		}

		pub const EDIT: &str = "workspace::edit";
		pub const DELETE: &str = "workspace::delete";
	}
}

#[allow(dead_code)]
#[api_macros::iterable_module(consts, recursive = false)]
pub mod resource_types {
	pub const WORKSPACE: &str = "workspace";
	pub const DOMAIN: &str = "domain";
	pub const DNS_RECORD: &str = "dnsRecord";
	pub const DOCKER_REPOSITORY: &str = "dockerRepository";
	pub const MANAGED_DATABASE: &str = "managedDatabase";
	pub const DEPLOYMENT: &str = "deployment";
	pub const STATIC_SITE: &str = "staticSite";
	pub const DEPLOYMENT_UPGRADE_PATH: &str = "deploymentUpgradePath";
	pub const MANAGED_URL: &str = "managedUrl";
	pub const SECRET: &str = "secret";
}
