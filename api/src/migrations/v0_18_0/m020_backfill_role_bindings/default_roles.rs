//! The default roles as they were seeded at the time of this migration.
//!
//! Deliberately a frozen copy rather than a call into
//! `routes::api_patr_cloud::workspace::create_workspace::default_roles()`.
//! A migration is a statement about one moment in the schema's history: it
//! must keep deciding what it decided the day it ran. Reading the live seed
//! list would mean a later edit there silently changes what this migration
//! marks immutable on databases that have not migrated yet.
//!
//! Stored as names rather than `Permission` values for the same reason — the
//! comparison is against `permission.name` in the database, so the live enum
//! can be renamed or extended without touching this.

/// A seeded role, as it was defined when this migration was written.
pub struct FrozenRole {
	/// Matched against `role.name`.
	pub name: &'static str,
	/// Used when the workspace is missing this role and it has to be created.
	pub description: &'static str,
	/// Its permission names, sorted to match the `array_agg(... ORDER BY
	/// p.name)` the caller compares against. The order is load-bearing — an
	/// entry listed out of order silently stops matching, and the role it
	/// describes quietly stays mutable — so `frozen_roles_are_sorted` guards
	/// it.
	pub permissions: &'static [&'static str],
}

/// The default roles seeded into every workspace as of v0.18.0.
///
/// Twenty-seven, not the thirty-six the seed once had: `m012` deleted the
/// `database`, `staticSite` and `dnsRecord` permissions and then every role
/// left holding none, which took the nine roles built purely out of them. By
/// the time this migration runs they are already gone, so listing them here
/// would only ever match nothing.
pub(super) const DEFAULT_ROLES: &[FrozenRole] = &[
	FrozenRole {
		name: "Deployment: Viewer",
		description: "Default role: read-only access to deployments.",
		permissions: &["deployment::view"],
	},
	FrozenRole {
		name: "Deployment: Editor",
		description: "Default role: create, edit, start, and stop deployments.",
		permissions: &[
			"deployment::create",
			"deployment::edit",
			"deployment::start",
			"deployment::stop",
			"deployment::view",
		],
	},
	FrozenRole {
		name: "Deployment: Admin",
		description: "Default role: full control over deployments, including delete.",
		permissions: &[
			"deployment::create",
			"deployment::delete",
			"deployment::edit",
			"deployment::start",
			"deployment::stop",
			"deployment::view",
		],
	},
	FrozenRole {
		name: "Volume: Viewer",
		description: "Default role: read-only access to volumes.",
		permissions: &["volume::view"],
	},
	FrozenRole {
		name: "Volume: Editor",
		description: "Default role: create and edit volumes.",
		permissions: &["volume::create", "volume::edit", "volume::view"],
	},
	FrozenRole {
		name: "Volume: Admin",
		description: "Default role: full control over volumes, including delete.",
		permissions: &[
			"volume::create",
			"volume::delete",
			"volume::edit",
			"volume::view",
		],
	},
	FrozenRole {
		name: "Secret: Viewer",
		description: "Default role: read-only access to secrets.",
		permissions: &["secret::view"],
	},
	FrozenRole {
		name: "Secret: Editor",
		description: "Default role: create and edit secrets.",
		permissions: &["secret::create", "secret::edit", "secret::view"],
	},
	FrozenRole {
		name: "Secret: Admin",
		description: "Default role: full control over secrets, including delete.",
		permissions: &[
			"secret::create",
			"secret::delete",
			"secret::edit",
			"secret::view",
		],
	},
	FrozenRole {
		name: "Managed URL: Viewer",
		description: "Default role: read-only access to managed URLs.",
		permissions: &["managedURL::verify", "managedURL::view"],
	},
	FrozenRole {
		name: "Managed URL: Editor",
		description: "Default role: add and edit managed URLs.",
		permissions: &[
			"managedURL::add",
			"managedURL::edit",
			"managedURL::verify",
			"managedURL::view",
		],
	},
	FrozenRole {
		name: "Managed URL: Admin",
		description: "Default role: full control over managed URLs, including delete.",
		permissions: &[
			"managedURL::add",
			"managedURL::delete",
			"managedURL::edit",
			"managedURL::verify",
			"managedURL::view",
		],
	},
	FrozenRole {
		name: "Domain: Viewer",
		description: "Default role: read-only access to domains.",
		permissions: &["domain::verify", "domain::view"],
	},
	FrozenRole {
		name: "Domain: Editor",
		description: "Default role: add and verify domains.",
		permissions: &["domain::add", "domain::verify", "domain::view"],
	},
	FrozenRole {
		name: "Domain: Admin",
		description: "Default role: full control over domains, including delete.",
		permissions: &[
			"domain::add",
			"domain::delete",
			"domain::verify",
			"domain::view",
		],
	},
	FrozenRole {
		name: "Runner: Viewer",
		description: "Default role: read-only access to runners.",
		permissions: &["runner::view"],
	},
	FrozenRole {
		name: "Runner: Editor",
		description: "Default role: create, edit, execute, and regenerate tokens on runners.",
		permissions: &[
			"runner::create",
			"runner::edit",
			"runner::execute",
			"runner::regenerateToken",
			"runner::view",
		],
	},
	FrozenRole {
		name: "Runner: Admin",
		description: "Default role: full control over runners, including delete.",
		permissions: &[
			"runner::create",
			"runner::delete",
			"runner::edit",
			"runner::execute",
			"runner::regenerateToken",
			"runner::view",
		],
	},
	FrozenRole {
		name: "Container Registry: Viewer",
		description: "Default role: read-only access to container registry repositories.",
		permissions: &[
			"containerRegistryRepository::pull",
			"containerRegistryRepository::view",
		],
	},
	FrozenRole {
		name: "Container Registry: Editor",
		description: "Default role: create, edit, and push to container registry repositories.",
		permissions: &[
			"containerRegistryRepository::create",
			"containerRegistryRepository::edit",
			"containerRegistryRepository::pull",
			"containerRegistryRepository::push",
			"containerRegistryRepository::view",
		],
	},
	FrozenRole {
		name: "Container Registry: Admin",
		description: "Default role: full control over container registry repositories, including delete.",
		permissions: &[
			"containerRegistryRepository::create",
			"containerRegistryRepository::delete",
			"containerRegistryRepository::deleteManifest",
			"containerRegistryRepository::edit",
			"containerRegistryRepository::pull",
			"containerRegistryRepository::push",
			"containerRegistryRepository::view",
		],
	},
	FrozenRole {
		name: "Billing: Viewer",
		description: "Default role: read-only access to billing.",
		permissions: &["billing::view"],
	},
	FrozenRole {
		name: "Billing: Editor",
		description: "Default role: edit billing details and make payments.",
		permissions: &["billing::edit", "billing::makePayment", "billing::view"],
	},
	FrozenRole {
		name: "Billing: Admin",
		description: "Default role: full control over billing.",
		permissions: &["billing::edit", "billing::makePayment", "billing::view"],
	},
	FrozenRole {
		name: "Workspace: Viewer",
		description: "Default role: view workspace roles.",
		permissions: &["viewRoles"],
	},
	FrozenRole {
		name: "Workspace: Editor",
		description: "Default role: view and modify workspace roles.",
		permissions: &["modifyRoles", "viewRoles"],
	},
	FrozenRole {
		name: "Workspace: Admin",
		description: "Default role: full workspace control, including role and workspace settings management.",
		permissions: &["editWorkspace", "modifyRoles", "viewRoles"],
	},
];

#[cfg(test)]
mod tests {
	use super::DEFAULT_ROLES;

	/// The permission comparison in `mark_default_roles_immutable` is
	/// order-sensitive, and nothing else would notice if an entry drifted.
	#[test]
	fn frozen_roles_are_sorted() {
		for role in DEFAULT_ROLES {
			let mut sorted = role.permissions.to_vec();
			sorted.sort_unstable();
			assert_eq!(
				role.permissions,
				sorted.as_slice(),
				"`{}` is not sorted",
				role.name
			);
		}
	}
}
