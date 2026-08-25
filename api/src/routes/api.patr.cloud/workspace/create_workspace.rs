use std::collections::HashMap;

use models::{
	api::workspace::*,
	rbac::{
		BillingPermission,
		ContainerRegistryRepositoryPermission,
		DatabasePermission,
		DeploymentPermission,
		DnsRecordPermission,
		DomainPermission,
		ManagedURLPermission,
		Permission,
		ResourcePermissionTypeDiscriminant,
		RunnerPermission,
		SecretPermission,
		StaticSitePermission,
		VolumePermission,
	},
};
use time::OffsetDateTime;

use crate::prelude::*;

/// The handler to create a new workspace. The workspace name must be unique.
/// Gated to `FeatureNotSupported` on self-hosted — a self-hosted instance is
/// single-workspace by design; the singleton is seeded out-of-band.
pub async fn create_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: CreateWorkspacePath,
				query: (),
				headers: CreateWorkspaceRequestHeaders {
					authorization,
					user_agent,
				},
				body: CreateWorkspaceRequestProcessed { name },
			},
		database,
		redis,
		client_ip,
		user_data,
		state,
	}: AuthenticatedAppRequest<'_, CreateWorkspaceRequest>,
) -> Result<AppResponse<CreateWorkspaceRequest>, ErrorType> {
	cfg_if! {
		if #[cfg(feature = "cloud")] {
			use axum::http::StatusCode;
			use rustis::commands::StringCommands;
			use std::ops::Add;

			info!("Creating workspace: `{name}`");

			let user_id = user_data.id;
			let available = super::is_name_available(AuthenticatedAppRequest {
				request: ProcessedApiRequest {
					path: IsWorkspaceNameAvailablePath,
					query: IsWorkspaceNameAvailableQueryProcessed {
						name: name.clone(),
					},
					headers: IsWorkspaceNameAvailableRequestHeaders {
						authorization,
						user_agent,
					},
					body: IsWorkspaceNameAvailableRequestProcessed,
				},
				database,
				redis,
				client_ip,
				user_data,
				state,
			})
			.await?
			.body
			.available;

			if !available {
				return Err(ErrorType::WorkspaceNameAlreadyExists);
			}

			query!(
				r#"
				SET CONSTRAINTS ALL DEFERRED;
				"#
			)
			.execute(&mut **database)
			.await?;

			// Create resource
			let workspace_id = query!(
				r#"
				INSERT INTO
					resource(
						id,
						resource_type_id,
						workspace_id,
						created
					)
				VALUES
					(
						GENERATE_RESOURCE_ID(),
						(SELECT id FROM resource_type WHERE name = 'workspace'),
						gen_random_uuid(),
						NOW()
					)
				RETURNING id AS "id: Uuid";
				"#,
			)
			.fetch_one(&mut **database)
			.await
			.map_err(|e| match e {
				sqlx::Error::Database(dbe) if dbe.is_unique_violation() => {
					ErrorType::ResourceAlreadyExists
				}
				other => other.into(),
			})?
			.id;

			// Create new workspace in database
			query!(
				r#"
				INSERT INTO
					workspace(
						id,
						name,
						super_admin_id,
						deleted
					)
				VALUES
					($1, $2, $3, NULL);
				"#,
				workspace_id as _,
				&name,
				user_id as _,
			)
			.execute(&mut **database)
			.await
			.map_err(|err| match err {
				sqlx::Error::Database(dbe) if dbe.is_unique_violation() => {
					ErrorType::WorkspaceNameAlreadyExists
				}
				other => other.into(),
			})?;

			// Update resource's owner to workspace id
			query!(
				r#"
				UPDATE
					resource
				SET
					workspace_id = $1
				WHERE
					id = $2;
				"#,
				workspace_id as _,
				workspace_id as _,
			)
			.execute(&mut **database)
			.await?;

			query!(
				r#"
				SET CONSTRAINTS ALL IMMEDIATE;
				"#
			)
			.execute(&mut **database)
			.await?;

			create_default_roles_for_workspace(&mut **database, &workspace_id).await?;

			// Revoke the token of the user who created the workspace
			redis
				.setex(
					redis::keys::user_id_revocation_timestamp(&user_id),
					constants::CACHED_PERMISSIONS_VALIDITY
						.whole_seconds()
						.unsigned_abs()
						.add(300),
					OffsetDateTime::now_utc().unix_timestamp_nanos().to_string(),
				)
				.await
				.inspect_err(|err| {
					error!("Error setting the revocation timestamp: `{}`", err);
				})?;

			AppResponse::builder()
				.body(CreateWorkspaceResponse {
					id: WithId::from(workspace_id),
				})
				.headers(())
				.status_code(StatusCode::CREATED)
				.build()
				.into_result()
		} else {
			let _ = (
				authorization,
				user_agent,
				name,
				database,
				redis,
				client_ip,
				user_data,
				state,
			);
			return Err(ErrorType::FeatureNotSupported);
		}
	}
}

/// A default role seeded into every new workspace. Also consumed by the
/// role-binding backfill migration to decide which existing roles are the
/// untouched seeded defaults (and therefore become immutable).
pub(crate) struct DefaultRole {
	/// The role's seeded name.
	pub(crate) name: &'static str,
	/// The role's seeded description.
	pub(crate) description: &'static str,
	/// The permissions the role grants (seeded as Exclude(∅) = whole
	/// workspace).
	pub(crate) permissions: Vec<Permission>,
}

/// The set of default roles seeded into every new workspace.
pub(crate) fn default_roles() -> Vec<DefaultRole> {
	use Permission::*;

	let mut roles = Vec::new();

	let view_db = vec![Database(DatabasePermission::View)];
	let edit_db = {
		let mut p = view_db.clone();
		p.extend([
			Database(DatabasePermission::Create),
			Database(DatabasePermission::Edit),
			Database(DatabasePermission::Backup),
			Database(DatabasePermission::Restore),
		]);
		p
	};
	let admin_db = {
		let mut p = edit_db.clone();
		p.push(Database(DatabasePermission::Delete));
		p
	};
	roles.push(DefaultRole {
		name: "Database: Viewer",
		description: "Default role: read-only access to databases.",
		permissions: view_db,
	});
	roles.push(DefaultRole {
		name: "Database: Editor",
		description: "Default role: create, edit, back up, and restore databases.",
		permissions: edit_db,
	});
	roles.push(DefaultRole {
		name: "Database: Admin",
		description: "Default role: full control over databases, including delete.",
		permissions: admin_db,
	});

	let view_deployment = vec![Deployment(DeploymentPermission::View)];
	let edit_deployment = {
		let mut p = view_deployment.clone();
		p.extend([
			Deployment(DeploymentPermission::Create),
			Deployment(DeploymentPermission::Edit),
			Deployment(DeploymentPermission::Start),
			Deployment(DeploymentPermission::Stop),
		]);
		p
	};
	let admin_deployment = {
		let mut p = edit_deployment.clone();
		p.push(Deployment(DeploymentPermission::Delete));
		p
	};
	roles.push(DefaultRole {
		name: "Deployment: Viewer",
		description: "Default role: read-only access to deployments.",
		permissions: view_deployment,
	});
	roles.push(DefaultRole {
		name: "Deployment: Editor",
		description: "Default role: create, edit, start, and stop deployments.",
		permissions: edit_deployment,
	});
	roles.push(DefaultRole {
		name: "Deployment: Admin",
		description: "Default role: full control over deployments, including delete.",
		permissions: admin_deployment,
	});

	let view_static = vec![StaticSite(StaticSitePermission::View)];
	let edit_static = {
		let mut p = view_static.clone();
		p.extend([
			StaticSite(StaticSitePermission::Create),
			StaticSite(StaticSitePermission::Edit),
			StaticSite(StaticSitePermission::Upload),
			StaticSite(StaticSitePermission::Start),
			StaticSite(StaticSitePermission::Stop),
		]);
		p
	};
	let admin_static = {
		let mut p = edit_static.clone();
		p.push(StaticSite(StaticSitePermission::Delete));
		p
	};
	roles.push(DefaultRole {
		name: "Static Site: Viewer",
		description: "Default role: read-only access to static sites.",
		permissions: view_static,
	});
	roles.push(DefaultRole {
		name: "Static Site: Editor",
		description: "Default role: create, edit, upload, start, and stop static sites.",
		permissions: edit_static,
	});
	roles.push(DefaultRole {
		name: "Static Site: Admin",
		description: "Default role: full control over static sites, including delete.",
		permissions: admin_static,
	});

	let view_volume = vec![Volume(VolumePermission::View)];
	let edit_volume = {
		let mut p = view_volume.clone();
		p.extend([
			Volume(VolumePermission::Create),
			Volume(VolumePermission::Edit),
		]);
		p
	};
	let admin_volume = {
		let mut p = edit_volume.clone();
		p.push(Volume(VolumePermission::Delete));
		p
	};
	roles.push(DefaultRole {
		name: "Volume: Viewer",
		description: "Default role: read-only access to volumes.",
		permissions: view_volume,
	});
	roles.push(DefaultRole {
		name: "Volume: Editor",
		description: "Default role: create and edit volumes.",
		permissions: edit_volume,
	});
	roles.push(DefaultRole {
		name: "Volume: Admin",
		description: "Default role: full control over volumes, including delete.",
		permissions: admin_volume,
	});

	let view_secret = vec![Secret(SecretPermission::View)];
	let edit_secret = {
		let mut p = view_secret.clone();
		p.extend([
			Secret(SecretPermission::Create),
			Secret(SecretPermission::Edit),
		]);
		p
	};
	let admin_secret = {
		let mut p = edit_secret.clone();
		p.push(Secret(SecretPermission::Delete));
		p
	};
	roles.push(DefaultRole {
		name: "Secret: Viewer",
		description: "Default role: read-only access to secrets.",
		permissions: view_secret,
	});
	roles.push(DefaultRole {
		name: "Secret: Editor",
		description: "Default role: create and edit secrets.",
		permissions: edit_secret,
	});
	roles.push(DefaultRole {
		name: "Secret: Admin",
		description: "Default role: full control over secrets, including delete.",
		permissions: admin_secret,
	});

	let view_managed_url = vec![
		ManagedURL(ManagedURLPermission::View),
		ManagedURL(ManagedURLPermission::Verify),
	];
	let edit_managed_url = {
		let mut p = view_managed_url.clone();
		p.extend([
			ManagedURL(ManagedURLPermission::Add),
			ManagedURL(ManagedURLPermission::Edit),
		]);
		p
	};
	let admin_managed_url = {
		let mut p = edit_managed_url.clone();
		p.push(ManagedURL(ManagedURLPermission::Delete));
		p
	};
	roles.push(DefaultRole {
		name: "Managed URL: Viewer",
		description: "Default role: read-only access to managed URLs.",
		permissions: view_managed_url,
	});
	roles.push(DefaultRole {
		name: "Managed URL: Editor",
		description: "Default role: add and edit managed URLs.",
		permissions: edit_managed_url,
	});
	roles.push(DefaultRole {
		name: "Managed URL: Admin",
		description: "Default role: full control over managed URLs, including delete.",
		permissions: admin_managed_url,
	});

	let view_domain = vec![
		Domain(DomainPermission::View),
		Domain(DomainPermission::Verify),
	];
	let edit_domain = {
		let mut p = view_domain.clone();
		p.push(Domain(DomainPermission::Add));
		p
	};
	let admin_domain = {
		let mut p = edit_domain.clone();
		p.push(Domain(DomainPermission::Delete));
		p
	};
	roles.push(DefaultRole {
		name: "Domain: Viewer",
		description: "Default role: read-only access to domains.",
		permissions: view_domain,
	});
	roles.push(DefaultRole {
		name: "Domain: Editor",
		description: "Default role: add and verify domains.",
		permissions: edit_domain,
	});
	roles.push(DefaultRole {
		name: "Domain: Admin",
		description: "Default role: full control over domains, including delete.",
		permissions: admin_domain,
	});

	let view_dns = vec![DnsRecord(DnsRecordPermission::View)];
	let edit_dns = {
		let mut p = view_dns.clone();
		p.extend([
			DnsRecord(DnsRecordPermission::Add),
			DnsRecord(DnsRecordPermission::Edit),
		]);
		p
	};
	let admin_dns = {
		let mut p = edit_dns.clone();
		p.push(DnsRecord(DnsRecordPermission::Delete));
		p
	};
	roles.push(DefaultRole {
		name: "DNS Record: Viewer",
		description: "Default role: read-only access to DNS records.",
		permissions: view_dns,
	});
	roles.push(DefaultRole {
		name: "DNS Record: Editor",
		description: "Default role: add and edit DNS records.",
		permissions: edit_dns,
	});
	roles.push(DefaultRole {
		name: "DNS Record: Admin",
		description: "Default role: full control over DNS records, including delete.",
		permissions: admin_dns,
	});

	let view_runner = vec![Runner(RunnerPermission::View)];
	let edit_runner = {
		let mut p = view_runner.clone();
		p.extend([
			Runner(RunnerPermission::Create),
			Runner(RunnerPermission::Edit),
			Runner(RunnerPermission::Execute),
			Runner(RunnerPermission::RegenerateToken),
		]);
		p
	};
	let admin_runner = {
		let mut p = edit_runner.clone();
		p.push(Runner(RunnerPermission::Delete));
		p
	};
	roles.push(DefaultRole {
		name: "Runner: Viewer",
		description: "Default role: read-only access to runners.",
		permissions: view_runner,
	});
	roles.push(DefaultRole {
		name: "Runner: Editor",
		description: "Default role: create, edit, execute, and regenerate tokens on runners.",
		permissions: edit_runner,
	});
	roles.push(DefaultRole {
		name: "Runner: Admin",
		description: "Default role: full control over runners, including delete.",
		permissions: admin_runner,
	});

	let view_registry = vec![
		ContainerRegistryRepository(ContainerRegistryRepositoryPermission::View),
		ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Pull),
	];
	let edit_registry = {
		let mut p = view_registry.clone();
		p.extend([
			ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Create),
			ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Edit),
			ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Push),
		]);
		p
	};
	let admin_registry = {
		let mut p = edit_registry.clone();
		p.extend([
			ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Delete),
			ContainerRegistryRepository(ContainerRegistryRepositoryPermission::DeleteManifest),
		]);
		p
	};
	roles.push(DefaultRole {
		name: "Container Registry: Viewer",
		description: "Default role: read-only access to container registry repositories.",
		permissions: view_registry,
	});
	roles.push(DefaultRole {
		name: "Container Registry: Editor",
		description: "Default role: create, edit, and push to container registry repositories.",
		permissions: edit_registry,
	});
	roles.push(DefaultRole {
		name: "Container Registry: Admin",
		description: "Default role: full control over container registry repositories, including delete.",
		permissions: admin_registry,
	});

	let view_billing = vec![Billing(BillingPermission::View)];
	let edit_billing = vec![
		Billing(BillingPermission::View),
		Billing(BillingPermission::Edit),
		Billing(BillingPermission::MakePayment),
	];
	let admin_billing = edit_billing.clone();
	roles.push(DefaultRole {
		name: "Billing: Viewer",
		description: "Default role: read-only access to billing.",
		permissions: view_billing,
	});
	roles.push(DefaultRole {
		name: "Billing: Editor",
		description: "Default role: edit billing details and make payments.",
		permissions: edit_billing,
	});
	roles.push(DefaultRole {
		name: "Billing: Admin",
		description: "Default role: full control over billing.",
		permissions: admin_billing,
	});

	let view_workspace = vec![ViewRoles];
	let edit_workspace = vec![ViewRoles, ModifyRoles];
	let admin_workspace = vec![ViewRoles, ModifyRoles, EditWorkspace];
	roles.push(DefaultRole {
		name: "Workspace: Viewer",
		description: "Default role: view workspace roles.",
		permissions: view_workspace,
	});
	roles.push(DefaultRole {
		name: "Workspace: Editor",
		description: "Default role: view and modify workspace roles.",
		permissions: edit_workspace,
	});
	roles.push(DefaultRole {
		name: "Workspace: Admin",
		description: "Default role: full workspace control, including role and workspace settings management.",
		permissions: admin_workspace,
	});

	roles
}

/// Creates the standard set of `Viewer` / `Editor` / `Admin` default roles for
/// every resource type in the workspace. These roles are seeded as ordinary
/// roles — they can be edited or deleted by workspace admins after creation.
#[instrument(skip(connection))]
async fn create_default_roles_for_workspace(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
) -> Result<(), ErrorType> {
	let permission_ids = query!(
		r#"
		SELECT
			id AS "id: Uuid",
			name
		FROM
			permission;
		"#,
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.name, row.id))
	.collect::<HashMap<_, _>>();

	let now = OffsetDateTime::now_utc();

	for role in default_roles() {
		let role_id = query!(
			r#"
			INSERT INTO
				resource(
					id,
					resource_type_id,
					workspace_id,
					created,
					deleted
				)
			VALUES
				(
					GENERATE_RESOURCE_ID(),
					(SELECT id FROM resource_type WHERE name = 'role'),
					$1,
					$2,
					NULL
				)
			RETURNING id AS "id: Uuid";
			"#,
			workspace_id as _,
			now as _,
		)
		.fetch_one(&mut *connection)
		.await
		.map_err(|err| match err {
			sqlx::Error::Database(err) if err.is_unique_violation() => ErrorType::RoleAlreadyExists,
			err => ErrorType::server_error(err),
		})?
		.id;

		query!(
			r#"
			INSERT INTO
				role(
					id,
					workspace_id,
					name,
					description
				)
			VALUES
				(
					$1,
					$2,
					$3,
					$4
				);
			"#,
			role_id as _,
			workspace_id as _,
			role.name,
			role.description,
		)
		.execute(&mut *connection)
		.await?;

		trace!("Role created. Inserting permissions.");

		for permission in role.permissions {
			let permission_name = permission.to_string();
			let permission_id = permission_ids.get(&permission_name).ok_or_else(|| {
				ErrorType::server_error(format!(
					"permission `{permission_name}` missing from permission table"
				))
			})?;
			let exclude_type = ResourcePermissionTypeDiscriminant::Exclude;

			query!(
				r#"
				INSERT INTO
					role_resource_permissions_type(
						role_id,
						permission_id,
						permission_type
					)
				VALUES
					(
						$1,
						$2,
						$3
					);
				"#,
				role_id as _,
				*permission_id as _,
				exclude_type as _,
			)
			.execute(&mut *connection)
			.await?;
		}
	}

	Ok(())
}
