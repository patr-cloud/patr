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
			pub const ADD: &str = "workspace::domain::add";
			pub const INFO: &str = "workspace::domain::info";
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
				pub const CREATE: &str =
					"workspace::infrastructure::deployment::create";
				pub const INFO: &str =
					"workspace::infrastructure::deployment::info";
				pub const DELETE: &str =
					"workspace::infrastructure::deployment::delete";
				pub const EDIT: &str =
					"workspace::infrastructure::deployment::edit";
			}

			pub mod managed_url {
				pub const INFO: &str =
					"workspace::infrastructure::managedUrl::info";
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
				pub const DELETE: &str =
					"workspace::infrastructure::managedDatabase::delete";
				pub const INFO: &str =
					"workspace::infrastructure::managedDatabase::info";
			}

			pub mod static_site {
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

		pub mod container_registry {
			pub const CREATE: &str = "workspace::containerRegistry::create";
			pub const DELETE: &str = "workspace::containerRegistry::delete";
			pub const INFO: &str = "workspace::containerRegistry::info";
			pub const PUSH: &str = "workspace::containerRegistry::push";
			pub const PULL: &str = "workspace::containerRegistry::pull";
		}

		pub mod secret {
			pub const INFO: &str = "workspace::secret::info";
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
			pub const INFO: &str = "workspace::region::info";
			pub const CHECK_STATUS: &str = "workspace::region::checkStatus";
			pub const ADD: &str = "workspace::region::add";
			pub const DELETE: &str = "workspace::region::delete";

			pub const LOKI_PUSH: &str = "workspace::region::loki_push";
		}

		pub mod ci {
			pub const RECENT_ACTIVITY: &str = "workspace::ci::recentActivity";

			pub mod git_provider {
				pub const LIST: &str = "workspace::ci::gitProvider::list";
				pub const CONNECT: &str = "workspace::ci::gitProvider::connect";
				pub const DISCONNECT: &str =
					"workspace::ci::gitProvider::disconnect";

				pub mod repo {
					pub const ACTIVATE: &str =
						"workspace::ci::gitProvider::repo::activate";
					pub const DEACTIVATE: &str =
						"workspace::ci::gitProvider::repo::deactivate";
					pub const SYNC: &str =
						"workspace::ci::gitProvider::repo::sync";
					pub const INFO: &str =
						"workspace::ci::gitProvider::repo::info";
					pub const WRITE: &str =
						"workspace::ci::gitProvider::repo::write";

					pub mod build {
						pub const LIST: &str =
							"workspace::ci::gitProvider::repo::build::list";
						pub const CANCEL: &str =
							"workspace::ci::gitProvider::repo::build::cancel";
						pub const INFO: &str =
							"workspace::ci::gitProvider::repo::build::info";
						pub const START: &str =
							"workspace::ci::gitProvider::repo::build::start";
						pub const RESTART: &str =
							"workspace::ci::gitProvider::repo::build::restart";
					}
				}
			}

			pub mod runner {
				pub const CREATE: &str = "workspace::ci::runner::create";
				pub const INFO: &str = "workspace::ci::runner::info";
				pub const UPDATE: &str = "workspace::ci::runner::update";
				pub const DELETE: &str = "workspace::ci::runner::delete";
			}
		}

		pub mod billing {
			pub const INFO: &str = "workspace::billing::info";
			pub const MAKE_PAYMENT: &str = "workspace::billing::makePayment";

			pub mod payment_method {
				pub const ADD: &str = "workspace::billing::paymentMethod::add";
				pub const DELETE: &str =
					"workspace::billing::paymentMethod::delete";
				pub const LIST: &str =
					"workspace::billing::paymentMethod::list";
				pub const EDIT: &str =
					"workspace::billing::paymentMethod::edit";
			}

			pub mod billing_address {
				pub const ADD: &str = "workspace::billing::billingAddress::add";
				pub const DELETE: &str =
					"workspace::billing::billingAddress::delete";
				pub const INFO: &str =
					"workspace::billing::billingAddress::info";
				pub const EDIT: &str =
					"workspace::billing::billingAddress::edit";
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
	pub const CONTAINER_REGISTRY_REPOSITORY: &str =
		"containerRegistryRepository";
	pub const MANAGED_DATABASE: &str = "managedDatabase";
	pub const DEPLOYMENT: &str = "deployment";
	pub const STATIC_SITE: &str = "staticSite";
	pub const MANAGED_URL: &str = "managedUrl";
	pub const SECRET: &str = "secret";
	pub const STATIC_SITE_UPLOAD: &str = "staticSiteUpload";
	pub const REGION: &str = "region";
	pub const DEPLOYMENT_VOLUME: &str = "deploymentVolume";

	// ci
	pub const CI_REPO: &str = "ciRepo";
	pub const CI_RUNNER: &str = "ciRunner";
}
