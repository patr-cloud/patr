//! Backfills the role-binding model from the legacy per-permission resource
//! lists, and proves the translation exact before anything reads it.
//!
//! Steps, in order:
//!
//! 1. `workspace_user_invite_role` gains a nullable `scope_id` (finalised to
//!    NOT NULL + new PK at cutover).
//! 2. One `actor` per distinct `(user, workspace)` membership pair.
//! 3. **Role split.** A legacy role holds a resource set per permission; a
//!    binding applies a whole role at one scope. Roles whose permissions carry
//!    different resource-set signatures are split — the largest group keeps the
//!    role row, each other group gets a new role named `"{orig} - {suffix}"`.
//!    The split is mirrored into the legacy tables and assignments so the old
//!    evaluator stays exactly equivalent.
//! 4. `is_immutable` for roles still matching what `default_roles()` seeds
//!    (name, permission set, all workspace-wide). Edited defaults stay mutable
//!    so nobody loses an ability they have today.
//! 5. `role_permission` from the legacy type rows.
//! 6. Bindings per assignment: `Exclude(∅)` → one workspace-scope binding;
//!    `Include(S)` → one per live same-workspace member of S; `Exclude(S≠∅)` →
//!    one per live workspace resource not in S (exact at cutover; resources
//!    created later are no longer auto-granted).
//! 7. Invite roles get scopes by the same expansion.
//! 8. Token ceilings: an owner binding is copied into a token's
//!    `api_token_role_binding` iff the token's declared grant covers every
//!    permission of the binding's role at the binding's scope. Declared
//!    narrowings with no role-shaped equivalent are dropped (count logged).
//! 9. **The proof**: the `(user, workspace, permission, resource)` tuple set
//!    derivable from the legacy tables (joined on both role_id AND
//!    permission_id — the corrected baseline) must equal the set derivable from
//!    bindings ⋈ role_permission, or the whole transaction aborts.

use std::collections::BTreeMap;

use sqlx::Row as _;

use crate::{prelude::*, routes::api_patr_cloud::workspace::create_workspace::default_roles};

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	invite_role_scope_ddl(&mut *connection).await?;
	mint_actors(&mut *connection).await?;
	split_non_uniform_roles(&mut *connection).await?;
	mark_immutable_default_roles(&mut *connection).await?;
	fill_role_permission(&mut *connection).await?;
	mint_bindings(&mut *connection).await?;
	expand_invite_scopes(&mut *connection).await?;
	backfill_token_ceilings(&mut *connection).await?;
	prove_equivalence(&mut *connection).await?;

	Ok(())
}

/// Adds the nullable `scope_id` to `workspace_user_invite_role`, replacing
/// the old PK with a NULLS NOT DISTINCT unique index so pre-expansion rows
/// keep their duplicate protection.
async fn invite_role_scope_ddl(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		ALTER TABLE workspace_user_invite_role
		ADD COLUMN scope_id UUID;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user_invite_role
		DROP CONSTRAINT workspace_user_invite_role_pk;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE UNIQUE INDEX
			workspace_user_invite_role_uq_invite_id_role_id_scope_id
		ON
			workspace_user_invite_role(invite_id, role_id, scope_id)
		NULLS NOT DISTINCT;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user_invite_role
		ADD CONSTRAINT workspace_user_invite_role_fk_scope_id_workspace_id
		FOREIGN KEY(scope_id, workspace_id) REFERENCES resource(id, workspace_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// One actor per distinct membership pair.
async fn mint_actors(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		INSERT INTO
			actor(id, workspace_id, actor_type, user_id, service_account_id)
		SELECT
			gen_random_uuid(),
			wu.workspace_id,
			'user',
			wu.user_id,
			NULL
		FROM
			(SELECT DISTINCT user_id, workspace_id FROM workspace_user) wu
		ON CONFLICT
			(user_id, workspace_id)
		DO NOTHING;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Splits every role whose permissions carry more than one resource-set
/// signature, mirroring the split into the legacy tables and assignments.
async fn split_non_uniform_roles(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	// Signature per (role, permission): mode plus the sorted resource list.
	let rows = sqlx::query(
		r#"
		SELECT
			t.role_id,
			t.permission_id,
			t.permission_type::TEXT || ':' || COALESCE(
				(
					SELECT string_agg(i.resource_id::TEXT, ',' ORDER BY i.resource_id)
					FROM role_resource_permissions_include i
					WHERE i.role_id = t.role_id AND i.permission_id = t.permission_id
				),
				(
					SELECT string_agg(e.resource_id::TEXT, ',' ORDER BY e.resource_id)
					FROM role_resource_permissions_exclude e
					WHERE e.role_id = t.role_id AND e.permission_id = t.permission_id
				),
				''
			) AS signature
		FROM
			role_resource_permissions_type t;
		"#,
	)
	.fetch_all(&mut *connection)
	.await?;

	// role_id -> signature -> permission ids
	let mut by_role = BTreeMap::<Uuid, BTreeMap<String, Vec<Uuid>>>::new();
	for row in rows {
		by_role
			.entry(row.try_get("role_id")?)
			.or_default()
			.entry(row.try_get("signature")?)
			.or_default()
			.push(row.try_get("permission_id")?);
	}

	let permission_names = sqlx::query(
		r#"
		SELECT
			id,
			name
		FROM
			permission;
		"#,
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| {
		Ok::<_, sqlx::Error>((
			row.try_get::<Uuid, _>("id")?,
			row.try_get::<String, _>("name")?,
		))
	})
	.collect::<Result<BTreeMap<_, _>, _>>()?;

	for (role_id, groups) in by_role {
		if groups.len() < 2 {
			continue;
		}

		let role = sqlx::query(
			r#"
			SELECT
				workspace_id,
				name,
				description
			FROM
				role
			WHERE
				id = $1;
			"#,
		)
		.bind(role_id)
		.fetch_one(&mut *connection)
		.await?;
		let workspace_id = role.try_get::<Uuid, _>("workspace_id")?;
		let orig_name = role.try_get::<String, _>("name")?;
		let description = role.try_get::<String, _>("description")?;

		// Largest group keeps the original role; deterministic order.
		let mut ordered = groups.into_iter().collect::<Vec<_>>();
		ordered.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));

		for (n, (_signature, permission_ids)) in ordered.into_iter().skip(1).enumerate() {
			let new_name = derive_split_name(
				&mut *connection,
				&workspace_id,
				&orig_name,
				&permission_ids,
				&permission_names,
				n,
			)
			.await?;

			let new_role_id = sqlx::query(
				r#"
				INSERT INTO
					resource(id, resource_type_id, workspace_id, created, deleted)
				VALUES
					(
						GENERATE_RESOURCE_ID(),
						(SELECT id FROM resource_type WHERE name = 'role'),
						$1,
						NOW(),
						NULL
					)
				RETURNING id;
				"#,
			)
			.bind(workspace_id)
			.fetch_one(&mut *connection)
			.await?
			.try_get::<Uuid, _>("id")?;

			sqlx::query(
				r#"
				INSERT INTO
					role(id, workspace_id, name, description, is_immutable)
				VALUES
					($1, $2, $3, $4, FALSE);
				"#,
			)
			.bind(new_role_id)
			.bind(workspace_id)
			.bind(&new_name)
			.bind(&description)
			.execute(&mut *connection)
			.await?;

			// Move this group's legacy rows: parent type rows first on
			// insert, children first on delete (the *_fk_parent FKs).
			sqlx::query(
				r#"
				INSERT INTO role_resource_permissions_type(role_id, permission_id, permission_type)
				SELECT $1, permission_id, permission_type
				FROM role_resource_permissions_type
				WHERE role_id = $2 AND permission_id = ANY($3);
				"#,
			)
			.bind(new_role_id)
			.bind(role_id)
			.bind(&permission_ids)
			.execute(&mut *connection)
			.await?;

			sqlx::query(
				r#"
				INSERT INTO role_resource_permissions_include(role_id, permission_id, resource_id)
				SELECT $1, permission_id, resource_id
				FROM role_resource_permissions_include
				WHERE role_id = $2 AND permission_id = ANY($3);
				"#,
			)
			.bind(new_role_id)
			.bind(role_id)
			.bind(&permission_ids)
			.execute(&mut *connection)
			.await?;

			sqlx::query(
				r#"
				INSERT INTO role_resource_permissions_exclude(role_id, permission_id, resource_id)
				SELECT $1, permission_id, resource_id
				FROM role_resource_permissions_exclude
				WHERE role_id = $2 AND permission_id = ANY($3);
				"#,
			)
			.bind(new_role_id)
			.bind(role_id)
			.bind(&permission_ids)
			.execute(&mut *connection)
			.await?;

			sqlx::query(
				r#"
				DELETE FROM role_resource_permissions_include
				WHERE role_id = $1 AND permission_id = ANY($2);
				"#,
			)
			.bind(role_id)
			.bind(&permission_ids)
			.execute(&mut *connection)
			.await?;

			sqlx::query(
				r#"
				DELETE FROM role_resource_permissions_exclude
				WHERE role_id = $1 AND permission_id = ANY($2);
				"#,
			)
			.bind(role_id)
			.bind(&permission_ids)
			.execute(&mut *connection)
			.await?;

			sqlx::query(
				r#"
				DELETE FROM role_resource_permissions_type
				WHERE role_id = $1 AND permission_id = ANY($2);
				"#,
			)
			.bind(role_id)
			.bind(&permission_ids)
			.execute(&mut *connection)
			.await?;

			// Everyone holding (or invited to) the original role also holds
			// the split-off role, keeping effective permissions identical.
			sqlx::query(
				r#"
				INSERT INTO workspace_user(user_id, workspace_id, role_id)
				SELECT user_id, workspace_id, $1
				FROM workspace_user
				WHERE role_id = $2;
				"#,
			)
			.bind(new_role_id)
			.bind(role_id)
			.execute(&mut *connection)
			.await?;

			sqlx::query(
				r#"
				INSERT INTO workspace_user_invite_role(invite_id, workspace_id, role_id)
				SELECT invite_id, workspace_id, $1
				FROM workspace_user_invite_role
				WHERE role_id = $2;
				"#,
			)
			.bind(new_role_id)
			.bind(role_id)
			.execute(&mut *connection)
			.await?;

			info!("Split role {orig_name} ({role_id}): {new_name} ({new_role_id})");
		}
	}

	Ok(())
}

/// Derives a name for a split-off role: `"{orig} - {prefix}"` when the
/// group's permissions share one resource-type prefix, else (and on any
/// collision or overflow) `"{orig} - {n}"`.
async fn derive_split_name(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	orig_name: &str,
	permission_ids: &[Uuid],
	permission_names: &BTreeMap<Uuid, String>,
	n: usize,
) -> Result<String, ErrorType> {
	let mut prefixes = permission_ids
		.iter()
		.filter_map(|id| permission_names.get(id))
		.map(|name| match name.split_once("::") {
			Some((prefix, _)) => prefix,
			// Bare variants (viewRoles, modifyRoles, editWorkspace).
			None => "workspace",
		})
		.collect::<Vec<_>>();
	prefixes.sort_unstable();
	prefixes.dedup();

	let mut candidates = Vec::new();
	if let [prefix] = prefixes.as_slice() {
		candidates.push(format!("{orig_name} - {prefix}"));
	}
	// Numeric fallbacks; n is this group's index so retries can't collide
	// with sibling groups of the same role.
	for k in 0.. {
		candidates.push(format!("{orig_name} - {}", n + 2 + k * 100));
		if k == 2 {
			break;
		}
	}

	for candidate in candidates {
		if candidate.len() > 100 {
			continue;
		}
		let taken = sqlx::query(
			r#"
			SELECT 1 AS present FROM role WHERE workspace_id = $1 AND name = $2;
			"#,
		)
		.bind(workspace_id)
		.bind(&candidate)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();
		if !taken {
			return Ok(candidate);
		}
	}

	Err(ErrorType::server_error(format!(
		"could not derive a unique name for a split of role `{orig_name}`"
	)))
}

/// Marks a role immutable only if its name and permission set still match
/// what `default_roles()` seeds, and every permission is workspace-wide.
async fn mark_immutable_default_roles(
	connection: &mut DatabaseConnection,
) -> Result<(), ErrorType> {
	for default_role in default_roles() {
		let mut names = default_role
			.permissions
			.iter()
			.map(ToString::to_string)
			.collect::<Vec<_>>();
		names.sort_unstable();

		sqlx::query(
			r#"
			UPDATE role r
			SET is_immutable = TRUE
			WHERE
				r.name = $1 AND
				NOT EXISTS (
					SELECT 1 FROM role_resource_permissions_include i WHERE i.role_id = r.id
				) AND
				NOT EXISTS (
					SELECT 1 FROM role_resource_permissions_exclude e WHERE e.role_id = r.id
				) AND
				NOT EXISTS (
					SELECT 1 FROM role_resource_permissions_type t
					WHERE t.role_id = r.id AND t.permission_type != 'exclude'
				) AND
				(
					SELECT COALESCE(array_agg(p.name::TEXT ORDER BY p.name::TEXT), '{}')
					FROM role_resource_permissions_type t
					INNER JOIN permission p ON p.id = t.permission_id
					WHERE t.role_id = r.id
				) = $2;
			"#,
		)
		.bind(default_role.name)
		.bind(&names)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}

/// A role's flat permission list, from the legacy type rows.
async fn fill_role_permission(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		INSERT INTO role_permission(role_id, permission_id)
		SELECT role_id, permission_id FROM role_resource_permissions_type
		ON CONFLICT DO NOTHING;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Mints bindings for every assignment. Post-split, every role is uniform,
/// so classification is per-role.
async fn mint_bindings(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	// Exclude(∅): the whole workspace, one binding at scope = workspace.
	sqlx::query(
		r#"
		INSERT INTO role_binding(id, workspace_id, actor_id, role_id, scope_id, created, created_by)
		SELECT
			gen_random_uuid(), wu.workspace_id, a.id, wu.role_id, wu.workspace_id, NOW(), NULL
		FROM workspace_user wu
		INNER JOIN actor a
			ON a.user_id = wu.user_id AND a.workspace_id = wu.workspace_id
		WHERE
			EXISTS (
				SELECT 1 FROM role_resource_permissions_type t
				WHERE t.role_id = wu.role_id AND t.permission_type = 'exclude'
			) AND
			NOT EXISTS (
				SELECT 1 FROM role_resource_permissions_exclude e WHERE e.role_id = wu.role_id
			)
		ON CONFLICT (actor_id, role_id, scope_id) DO NOTHING;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Include(S): one binding per live, same-workspace member of S.
	// Cross-workspace include rows are dead grants under the corrected
	// evaluator; deleted resources cannot come back.
	sqlx::query(
		r#"
		INSERT INTO role_binding(id, workspace_id, actor_id, role_id, scope_id, created, created_by)
		SELECT
			gen_random_uuid(), wu.workspace_id, a.id, wu.role_id, i.resource_id, NOW(), NULL
		FROM workspace_user wu
		INNER JOIN actor a
			ON a.user_id = wu.user_id AND a.workspace_id = wu.workspace_id
		INNER JOIN (
			SELECT DISTINCT role_id, resource_id FROM role_resource_permissions_include
		) i ON i.role_id = wu.role_id
		INNER JOIN resource r
			ON r.id = i.resource_id AND r.workspace_id = wu.workspace_id AND
				r.deleted IS NULL AND r.id <> r.workspace_id
		ON CONFLICT (actor_id, role_id, scope_id) DO NOTHING;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Exclude(S≠∅): one binding per live workspace resource not in S.
	sqlx::query(
		r#"
		INSERT INTO role_binding(id, workspace_id, actor_id, role_id, scope_id, created, created_by)
		SELECT
			gen_random_uuid(), wu.workspace_id, a.id, wu.role_id, r.id, NOW(), NULL
		FROM workspace_user wu
		INNER JOIN actor a
			ON a.user_id = wu.user_id AND a.workspace_id = wu.workspace_id
		INNER JOIN resource r
			ON r.workspace_id = wu.workspace_id AND r.deleted IS NULL AND
				r.id <> r.workspace_id
		WHERE
			EXISTS (
				SELECT 1 FROM role_resource_permissions_exclude e WHERE e.role_id = wu.role_id
			) AND
			NOT EXISTS (
				SELECT 1 FROM role_resource_permissions_exclude e2
				WHERE e2.role_id = wu.role_id AND e2.resource_id = r.id
			)
		ON CONFLICT (actor_id, role_id, scope_id) DO NOTHING;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Gives every invite role row a scope, by the same expansion as bindings.
/// Operates only on `scope_id IS NULL` rows, in strict order.
async fn expand_invite_scopes(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	// 1. Workspace scope for Exclude(∅) roles.
	sqlx::query(
		r#"
		UPDATE workspace_user_invite_role ir
		SET scope_id = ir.workspace_id
		WHERE
			ir.scope_id IS NULL AND
			EXISTS (
				SELECT 1 FROM role_resource_permissions_type t
				WHERE t.role_id = ir.role_id AND t.permission_type = 'exclude'
			) AND
			NOT EXISTS (
				SELECT 1 FROM role_resource_permissions_exclude e WHERE e.role_id = ir.role_id
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// 2. Include expansion.
	sqlx::query(
		r#"
		INSERT INTO workspace_user_invite_role(invite_id, workspace_id, role_id, scope_id)
		SELECT ir.invite_id, ir.workspace_id, ir.role_id, i.resource_id
		FROM workspace_user_invite_role ir
		INNER JOIN (
			SELECT DISTINCT role_id, resource_id FROM role_resource_permissions_include
		) i ON i.role_id = ir.role_id
		INNER JOIN resource r
			ON r.id = i.resource_id AND r.workspace_id = ir.workspace_id AND
				r.deleted IS NULL AND r.id <> r.workspace_id
		WHERE ir.scope_id IS NULL
		ON CONFLICT (invite_id, role_id, scope_id) DO NOTHING;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// 3. Exclude(S≠∅) expansion.
	sqlx::query(
		r#"
		INSERT INTO workspace_user_invite_role(invite_id, workspace_id, role_id, scope_id)
		SELECT ir.invite_id, ir.workspace_id, ir.role_id, r.id
		FROM workspace_user_invite_role ir
		INNER JOIN resource r
			ON r.workspace_id = ir.workspace_id AND r.deleted IS NULL AND
				r.id <> r.workspace_id
		WHERE
			ir.scope_id IS NULL AND
			EXISTS (
				SELECT 1 FROM role_resource_permissions_exclude e WHERE e.role_id = ir.role_id
			) AND
			NOT EXISTS (
				SELECT 1 FROM role_resource_permissions_exclude e2
				WHERE e2.role_id = ir.role_id AND e2.resource_id = r.id
			)
		ON CONFLICT (invite_id, role_id, scope_id) DO NOTHING;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// 4. Drop the expanded NULL originals (workspace-scope rows were
	// UPDATEd in step 1, so they no longer match). An include-only invite
	// role whose entire list is dead loses its row here — its grant was
	// empty anyway, and membership-on-accept becomes unconditional at
	// cutover.
	sqlx::query(
		r#"
		DELETE FROM workspace_user_invite_role ir
		WHERE
			ir.scope_id IS NULL AND
			EXISTS (
				SELECT 1 FROM role_resource_permissions_type t WHERE t.role_id = ir.role_id
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// 5. Zero-permission roles: membership-only grant, workspace scope.
	sqlx::query(
		r#"
		UPDATE workspace_user_invite_role
		SET scope_id = workspace_id
		WHERE scope_id IS NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Copies an owner binding into a token's ceiling iff the token's declared
/// grant covers every permission of the binding's role at the binding's
/// scope — never widening a token. Logs how many tokens lose narrowing.
async fn backfill_token_ceilings(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		INSERT INTO api_token_role_binding(token_id, workspace_id, role_id, scope_id)
		SELECT tk.token_id, b.workspace_id, b.role_id, b.scope_id
		FROM user_api_token tk
		INNER JOIN actor a ON a.actor_type = 'user' AND a.user_id = tk.user_id
		INNER JOIN role_binding b ON b.actor_id = a.id
		WHERE
			EXISTS (
				SELECT 1 FROM user_api_token_resource_permissions_type d
				WHERE d.token_id = tk.token_id AND d.workspace_id = b.workspace_id
			) AND
			NOT EXISTS (
				SELECT 1
				FROM role_permission rp
				WHERE rp.role_id = b.role_id AND NOT (
					CASE WHEN b.scope_id = b.workspace_id THEN
						EXISTS (
							SELECT 1 FROM user_api_token_resource_permissions_type d
							WHERE
								d.token_id = tk.token_id AND
								d.workspace_id = b.workspace_id AND
								d.permission_id = rp.permission_id AND
								d.resource_permission_type = 'exclude' AND
								NOT EXISTS (
									SELECT 1 FROM user_api_token_resource_permissions_exclude ex
									WHERE
										ex.token_id = d.token_id AND
										ex.workspace_id = d.workspace_id AND
										ex.permission_id = d.permission_id
								)
						)
					ELSE
						EXISTS (
							SELECT 1 FROM user_api_token_resource_permissions_type d
							WHERE
								d.token_id = tk.token_id AND
								d.workspace_id = b.workspace_id AND
								d.permission_id = rp.permission_id AND
								(
									(
										d.resource_permission_type = 'include' AND
										EXISTS (
											SELECT 1
											FROM user_api_token_resource_permissions_include inc
											WHERE
												inc.token_id = d.token_id AND
												inc.workspace_id = d.workspace_id AND
												inc.permission_id = d.permission_id AND
												inc.resource_id = b.scope_id
										)
									) OR
									(
										d.resource_permission_type = 'exclude' AND
										NOT EXISTS (
											SELECT 1
											FROM user_api_token_resource_permissions_exclude ex
											WHERE
												ex.token_id = d.token_id AND
												ex.workspace_id = d.workspace_id AND
												ex.permission_id = d.permission_id AND
												ex.resource_id = b.scope_id
										)
									)
								)
						)
					END
				)
			)
		ON CONFLICT (token_id, role_id, scope_id) DO NOTHING;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	let tokens_losing_narrowing = sqlx::query(
		r#"
		WITH declared AS (
			SELECT inc.token_id, inc.workspace_id, inc.permission_id, r.id AS resource_id
			FROM user_api_token_resource_permissions_include inc
			INNER JOIN resource r
				ON r.id = inc.resource_id AND r.workspace_id = inc.workspace_id AND
					r.deleted IS NULL
			UNION
			SELECT t.token_id, t.workspace_id, t.permission_id, r.id
			FROM user_api_token_resource_permissions_type t
			INNER JOIN resource r
				ON r.workspace_id = t.workspace_id AND r.deleted IS NULL
			WHERE
				t.resource_permission_type = 'exclude' AND
				NOT EXISTS (
					SELECT 1 FROM user_api_token_resource_permissions_exclude ex
					WHERE
						ex.token_id = t.token_id AND
						ex.workspace_id = t.workspace_id AND
						ex.permission_id = t.permission_id AND
						ex.resource_id = r.id
				)
		),
		owner_grant AS (
			SELECT tk.token_id, b.workspace_id, rp.permission_id, r.id AS resource_id
			FROM user_api_token tk
			INNER JOIN actor a ON a.actor_type = 'user' AND a.user_id = tk.user_id
			INNER JOIN role_binding b ON b.actor_id = a.id
			INNER JOIN role_permission rp ON rp.role_id = b.role_id
			INNER JOIN resource r
				ON r.workspace_id = b.workspace_id AND r.deleted IS NULL AND
					(b.scope_id = b.workspace_id OR r.id = b.scope_id)
		),
		ceiling_grant AS (
			SELECT atrb.token_id, atrb.workspace_id, rp.permission_id, r.id AS resource_id
			FROM api_token_role_binding atrb
			INNER JOIN role_permission rp ON rp.role_id = atrb.role_id
			INNER JOIN resource r
				ON r.workspace_id = atrb.workspace_id AND r.deleted IS NULL AND
					(atrb.scope_id = atrb.workspace_id OR r.id = atrb.scope_id)
		),
		lost AS (
			SELECT * FROM (
				SELECT * FROM declared
				INTERSECT
				SELECT * FROM owner_grant
			) covered
			EXCEPT
			SELECT * FROM ceiling_grant
		)
		SELECT COUNT(DISTINCT token_id) AS tokens FROM lost;
		"#,
	)
	.fetch_one(&mut *connection)
	.await?
	.try_get::<i64, _>("tokens")?;

	info!("Token ceiling backfill: {tokens_losing_narrowing} token(s) lose declared narrowing");

	Ok(())
}

/// Aborts the transaction unless the legacy grant tuples and the binding
/// tuples are identical over live resources.
///
/// One divergence class is accepted and only counted: a grant whose target
/// is the workspace's own resource row, held through a non-uniform-scope
/// role (Include(S) / Exclude(S≠∅)). The new encoding cannot say "on the
/// workspace itself but not workspace-wide" — `scope_id = workspace_id`
/// *means* workspace-wide — so the expansions skip that row (fail-safe
/// under-grant) rather than silently widening the grant to everything.
async fn prove_equivalence(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	let row = sqlx::query(
		r#"
		WITH old_tuples AS (
			SELECT wu.user_id, wu.workspace_id, i.permission_id, r.id AS resource_id
			FROM workspace_user wu
			INNER JOIN role_resource_permissions_include i
				ON i.role_id = wu.role_id
			INNER JOIN resource r
				ON r.id = i.resource_id AND r.workspace_id = wu.workspace_id AND
					r.deleted IS NULL
			UNION
			SELECT wu.user_id, wu.workspace_id, t.permission_id, r.id
			FROM workspace_user wu
			INNER JOIN role_resource_permissions_type t
				ON t.role_id = wu.role_id AND t.permission_type = 'exclude'
			INNER JOIN resource r
				ON r.workspace_id = wu.workspace_id AND r.deleted IS NULL
			WHERE NOT EXISTS (
				SELECT 1 FROM role_resource_permissions_exclude e
				WHERE
					e.role_id = wu.role_id AND
					e.permission_id = t.permission_id AND
					e.resource_id = r.id
			)
		),
		new_tuples AS (
			SELECT a.user_id, b.workspace_id, rp.permission_id, r.id AS resource_id
			FROM role_binding b
			INNER JOIN actor a ON a.id = b.actor_id
			INNER JOIN role_permission rp ON rp.role_id = b.role_id
			INNER JOIN resource r
				ON r.workspace_id = b.workspace_id AND r.deleted IS NULL AND
					(b.scope_id = b.workspace_id OR r.id = b.scope_id)
		),
		missing AS (
			SELECT * FROM old_tuples EXCEPT SELECT * FROM new_tuples
		),
		extra AS (
			SELECT * FROM new_tuples EXCEPT SELECT * FROM old_tuples
		)
		SELECT
			(
				SELECT COUNT(*) FROM missing
				WHERE missing.resource_id = missing.workspace_id
			) AS "accepted",
			(
				SELECT COUNT(*) FROM missing
				WHERE missing.resource_id <> missing.workspace_id
			) AS "missing",
			(SELECT COUNT(*) FROM extra) AS "extra";
		"#,
	)
	.fetch_one(&mut *connection)
	.await?;

	let accepted = row.try_get::<i64, _>("accepted")?;
	let missing = row.try_get::<i64, _>("missing")?;
	let extra = row.try_get::<i64, _>("extra")?;

	if missing > 0 || extra > 0 {
		return Err(ErrorType::server_error(format!(
			"role_binding backfill diverges from legacy grants: {missing} tuple(s) lost, \
			 {extra} tuple(s) gained (user, workspace, permission, resource)"
		)));
	}

	info!("Backfill equivalence proven; {accepted} accepted workspace-target under-grant tuple(s)");

	Ok(())
}
