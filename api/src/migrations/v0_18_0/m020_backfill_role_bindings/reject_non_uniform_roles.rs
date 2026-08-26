use std::collections::{BTreeMap, BTreeSet};

use sqlx::Row as _;

use crate::prelude::*;

/// Aborts unless every role's permissions all target the same resources.
///
/// The old model put the target on each permission, so one role could grant
/// `deployment::edit` on six deployments and `deployment::start` on one. A
/// binding carries one scope for the whole role, so such a role has no
/// representation, and neither guess is safe — the union widens the narrow
/// permission, the intersection takes access away. Refuse, and name what to
/// split.
pub(super) async fn reject_non_uniform_roles(
	connection: &mut DatabaseConnection,
) -> Result<(), ErrorType> {
	// A permission is either an include or an exclude, never both, so only one
	// of the two joined columns is ever non-null for a given row.
	let rows = sqlx::query(
		r#"
		SELECT
			workspace.name AS workspace_name,
			role.name AS role_name,
			permission_type.permission_id,
			permission_type.permission_type::TEXT AS kind,
			COALESCE(included.resource_id, excluded.resource_id) AS resource_id
		FROM
			role_resource_permissions_type permission_type
		INNER JOIN
			role
		ON
			role.id = permission_type.role_id
		INNER JOIN
			workspace
		ON
			workspace.id = role.workspace_id
		LEFT JOIN
			role_resource_permissions_include included
		ON
			included.role_id = permission_type.role_id AND
			included.permission_id = permission_type.permission_id
		LEFT JOIN
			role_resource_permissions_exclude excluded
		ON
			excluded.role_id = permission_type.role_id AND
			excluded.permission_id = permission_type.permission_id;
		"#,
	)
	.fetch_all(&mut *connection)
	.await?;

	let mut scopes = BTreeMap::<(String, String), BTreeMap<Uuid, (String, BTreeSet<Uuid>)>>::new();
	for row in rows {
		let role = (
			row.try_get::<String, _>("workspace_name")?,
			row.try_get::<String, _>("role_name")?,
		);
		let permission_id = row.try_get::<Uuid, _>("permission_id")?;
		let kind = row.try_get::<String, _>("kind")?;
		let resource_id = row.try_get::<Option<Uuid>, _>("resource_id")?;

		let entry = scopes
			.entry(role)
			.or_default()
			.entry(permission_id)
			.or_insert((kind, BTreeSet::new()));
		if let Some(resource_id) = resource_id {
			entry.1.insert(resource_id);
		}
	}

	let offenders = scopes
		.into_iter()
		.filter(|(_, permissions)| permissions.values().collect::<BTreeSet<_>>().len() > 1)
		.map(|((workspace, role), _)| format!("{workspace} / {role}"))
		.collect::<Vec<_>>();

	if !offenders.is_empty() {
		return Err(ErrorType::server_error(format!(
			"cannot migrate: these roles target different resources per permission, so they have \
			 no single scope. Split each into one role per target set, then re-run: {}",
			offenders.join(", ")
		)));
	}

	Ok(())
}
