//! Removes static sites, managed databases and DNS records.
//!
//! Static sites and managed databases were never implemented — their handlers
//! were `todo!()` stubs and their route modules were commented out — so the
//! tables only ever held rows a fresh workspace seeded. DNS records lost their
//! table back in `m007_unify_domains`; what survived was RBAC metadata for a
//! resource that no longer existed.
//!
//! The destructive parts, in order:
//!
//! - Managed URLs of type `proxy_to_static_site` are deleted. The static sites they proxy to are
//!   going away, so there is nothing left for them to serve.
//! - `permission` and `resource_type` rows for the three removed resources are deleted, cascading
//!   through the `role_resource_permissions_*` tables.
//! - Any role left holding no permissions at all is deleted, along with the `workspace_user` rows
//!   assigning it. A user whose only role was one of these loses their workspace membership.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	delete_static_site_managed_urls(&mut *connection).await?;
	drop_static_site_column(&mut *connection).await?;
	drop_resource_tables(&mut *connection).await?;
	purge_rbac_metadata(&mut *connection).await?;
	delete_empty_roles(&mut *connection).await?;

	Ok(())
}

/// Deletes every managed URL that proxies to a static site. This has to happen
/// before the CHECK constraint is rewritten, or the surviving rows would fail
/// the new constraint the moment it is added.
async fn delete_static_site_managed_urls(
	connection: &mut DatabaseConnection,
) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		DELETE FROM
			managed_url
		WHERE
			url_type = 'proxy_to_static_site';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Drops `managed_url.static_site_id` along with the FK and the CHECK arm that
/// referenced it, then rebuilds the CHECK over the three remaining URL types.
///
/// `MANAGED_URL_TYPE` keeps its `proxy_to_static_site` value: Postgres cannot
/// remove a value from an enum, and rewriting the type would mean rebuilding
/// every column that uses it. The CHECK constraint is what actually prevents
/// new rows from using it.
async fn drop_static_site_column(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		ALTER TABLE managed_url
			DROP CONSTRAINT managed_url_fk_static_site_id_workspace_id,
			DROP CONSTRAINT managed_url_chk_values_null_or_not_null;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE managed_url
			DROP COLUMN static_site_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE managed_url
			ADD CONSTRAINT managed_url_chk_values_null_or_not_null CHECK(
				(
					url_type = 'proxy_to_deployment' AND
					deployment_id IS NOT NULL AND
					port IS NOT NULL AND
					url IS NULL AND
					permanent_redirect IS NULL AND
					http_only IS NULL
				) OR (
					url_type = 'proxy_url' AND
					deployment_id IS NULL AND
					port IS NULL AND
					url IS NOT NULL AND
					permanent_redirect IS NULL AND
					http_only IS NOT NULL
				) OR (
					url_type = 'redirect' AND
					deployment_id IS NULL AND
					port IS NULL AND
					url IS NOT NULL AND
					permanent_redirect IS NOT NULL AND
					http_only IS NOT NULL
				)
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Drops the static site and managed database tables and their enum types.
/// `resource` rows for these are removed first, since `resource` is what the
/// RBAC tables key off.
async fn drop_resource_tables(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		DELETE FROM
			role_resource_permissions_include
		WHERE
			resource_id IN (
				SELECT id FROM static_site
				UNION ALL
				SELECT id FROM managed_database
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			role_resource_permissions_exclude
		WHERE
			resource_id IN (
				SELECT id FROM static_site
				UNION ALL
				SELECT id FROM managed_database
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			resource
		WHERE
			id IN (
				SELECT id FROM static_site
				UNION ALL
				SELECT id FROM managed_database
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Dropped in a single statement: `static_site` and `static_site_upload_history`
	// reference each other (`static_site_fk_current_live_upload` points at the
	// history table, which in turn FKs back to the site), so neither can be
	// dropped on its own.
	sqlx::query(
		r#"
		DROP TABLE IF EXISTS
			static_site_upload_history,
			static_site,
			managed_database,
			managed_database_plan;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TYPE IF EXISTS MANAGED_DATABASE_STATUS;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TYPE IF EXISTS MANAGED_DATABASE_ENGINE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TYPE IF EXISTS LEGACY_MANAGED_DATABASE_PLAN;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Deletes the `permission` and `resource_type` rows for the removed
/// resources. Permission names are the `Display` form of the old enum
/// variants, e.g. `staticSite::create`, so matching on the prefix catches every
/// variant without listing them.
async fn purge_rbac_metadata(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	for table in [
		"role_resource_permissions_include",
		"role_resource_permissions_exclude",
		"role_resource_permissions_type",
	] {
		sqlx::query(&format!(
			r#"
			DELETE FROM
				{table}
			WHERE
				permission_id IN (
					SELECT
						id
					FROM
						permission
					WHERE
						name LIKE 'staticSite::%' OR
						name LIKE 'database::%' OR
						name LIKE 'dnsRecord::%'
				);
			"#
		))
		.execute(&mut *connection)
		.await?;
	}

	sqlx::query(
		r#"
		DELETE FROM
			permission
		WHERE
			name LIKE 'staticSite::%' OR
			name LIKE 'database::%' OR
			name LIKE 'dnsRecord::%';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			resource_type
		WHERE
			name IN ('staticSite', 'database', 'dnsRecord');
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Deletes roles that hold no permissions after the purge above, and everything
/// that referenced them. This catches the nine default roles seeded for the
/// removed resources, and any custom role whose entire permission set was made
/// up of them.
///
/// Three things FK onto `role`: `workspace_user`, `workspace_user_invite_role`
/// and `role_resource_permissions_type` (already empty for these roles by
/// definition). Pending invites are cleared first, and an invite left granting
/// no roles at all is deleted outright — accepting it would grant nothing.
async fn delete_empty_roles(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		DELETE FROM
			workspace_user_invite_role
		WHERE
			role_id IN (
				SELECT
					role.id
				FROM
					role
				LEFT JOIN
					role_resource_permissions_type
				ON
					role_resource_permissions_type.role_id = role.id
				WHERE
					role_resource_permissions_type.role_id IS NULL
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			workspace_user_invite
		WHERE
			NOT EXISTS (
				SELECT
					1
				FROM
					workspace_user_invite_role
				WHERE
					workspace_user_invite_role.invite_id = workspace_user_invite.id
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			workspace_user
		WHERE
			role_id IN (
				SELECT
					role.id
				FROM
					role
				LEFT JOIN
					role_resource_permissions_type
				ON
					role_resource_permissions_type.role_id = role.id
				WHERE
					role_resource_permissions_type.role_id IS NULL
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			role
		WHERE
			id IN (
				SELECT
					role.id
				FROM
					role
				LEFT JOIN
					role_resource_permissions_type
				ON
					role_resource_permissions_type.role_id = role.id
				WHERE
					role_resource_permissions_type.role_id IS NULL
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
