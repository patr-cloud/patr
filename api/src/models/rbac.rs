use std::collections::HashMap;

use api_models::utils::Uuid;
use once_cell::sync::OnceCell;

pub static GOD_USER_ID: OnceCell<Uuid> = OnceCell::new();
// A mapping of resource type name -> resource type IDs
pub static RESOURCE_TYPES: OnceCell<HashMap<String, Uuid>> = OnceCell::new();
// A mapping of permission names -> permission IDs
pub static PERMISSIONS: OnceCell<HashMap<String, Uuid>> = OnceCell::new();

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

			pub mod patr_database {
				pub const CREATE: &str =
					"workspace::infrastructure::patrDatabase::create";
				pub const LIST: &str =
					"workspace::infrastructure::patrDatabase::list";
				pub const DELETE: &str =
					"workspace::infrastructure::patrDatabase::delete";
				pub const INFO: &str =
					"workspace::infrastructure::patrDatabase::info";
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

		pub mod region {
			pub const LIST: &str = "workspace::region::list";
			pub const INFO: &str = "workspace::region::info";
			pub const CHECK_STATUS: &str = "workspace::region::check_status";
			pub const ADD: &str = "workspace::region::add";
			pub const DELETE: &str = "workspace::region::delete";
		}

		pub mod ci {
			pub const RECENT_ACTIVITY: &str = "workspace::ci::recent_activity";

			pub mod git_provider {
				pub const LIST: &str = "workspace::ci::git_provider::list";
				pub const CONNECT: &str =
					"workspace::ci::git_provider::connect";
				pub const DISCONNECT: &str =
					"workspace::ci::git_provider::disconnect";

				pub mod repo {
					pub const ACTIVATE: &str =
						"workspace::ci::git_provider::repo::activate";
					pub const DEACTIVATE: &str =
						"workspace::ci::git_provider::repo::deactivate";
					pub const LIST: &str =
						"workspace::ci::git_provider::repo::list";
					pub const INFO: &str =
						"workspace::ci::git_provider::repo::info";
					pub const WRITE: &str =
						"workspace::ci::git_provider::repo::write";

					pub mod build {
						pub const LIST: &str =
							"workspace::ci::git_provider::repo::build::list";
						pub const CANCEL: &str =
							"workspace::ci::git_provider::repo::build::cancel";
						pub const INFO: &str =
							"workspace::ci::git_provider::repo::build::info";
						pub const START: &str =
							"workspace::ci::git_provider::repo::build::start";
						pub const RESTART: &str =
							"workspace::ci::git_provider::repo::build::restart";
					}
				}
			}

			pub mod runner {
				pub const LIST: &str = "workspace::ci::runner::list";
				pub const CREATE: &str = "workspace::ci::runner::create";
				pub const INFO: &str = "workspace::ci::runner::info";
				pub const UPDATE: &str = "workspace::ci::runner::update";
				pub const DELETE: &str = "workspace::ci::runner::delete";
			}
		}

		pub mod billing {
			pub const INFO: &str = "workspace::billing::info";
			pub const MAKE_PAYMENT: &str = "workspace::billing::make_payment";

			pub mod payment_method {
				pub const ADD: &str = "workspace::billing::payment_method::add";
				pub const DELETE: &str =
					"workspace::billing::payment_method::delete";
				pub const LIST: &str =
					"workspace::billing::payment_method::list";
				pub const EDIT: &str =
					"workspace::billing::payment_method::edit";
			}

			pub mod billing_address {
				pub const ADD: &str =
					"workspace::billing::billing_address::add";
				pub const DELETE: &str =
					"workspace::billing::billing_address::delete";
				pub const INFO: &str =
					"workspace::billing::billing_address::info";
				pub const EDIT: &str =
					"workspace::billing::billing_address::edit";
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
	pub const STATIC_SITE_UPLOAD: &str = "staticSiteUpload";
	pub const DEPLOYMENT_REGION: &str = "deploymentRegion";
	pub const DEPLOYMENT_VOLUME: &str = "deploymentVolume";
	pub const PATR_DATABASE: &str = "patrDatabase";

	// ci
	pub const CI_REPO: &str = "ciRepo";
	pub const CI_RUNNER: &str = "ciRunner";
}
