use std::collections::{BTreeMap, BTreeSet};

use axum::http::StatusCode;
use models::{api::workspace::service_account::*, rbac::WorkspacePermission};
use sha2::{Digest as _, Sha256};

use crate::{models::permissions, prelude::*};

pub async fn regenerate_service_account_token(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					RegenerateServiceAccountTokenPath {
						workspace_id,
						service_account_id,
					},
				query: (),
				headers:
					RegenerateServiceAccountTokenRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: RegenerateServiceAccountTokenRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		actor_data,
		mut state,
	}: AuthenticatedAppRequest<'_, RegenerateServiceAccountTokenRequest>,
) -> Result<AppResponse<RegenerateServiceAccountTokenRequest>, ErrorType> {
	// Joined to the permissions these bindings grant, to check the caller holds them.
	let granted = query!(
		r#"
		SELECT
			role_permission.permission_id,
			role_binding.scope_id
		FROM
			role_binding
		INNER JOIN
			role_permission
		ON
			role_permission.role_id = role_binding.role_id
		WHERE
			role_binding.actor_id = $1;
		"#,
		service_account_id,
	)
	.fetch_all(&mut **database)
	.await?;

	if !actor_data
		.permissions
		.get(&workspace_id)
		.is_some_and(|held| {
			held.is_superset_of(
				&WorkspacePermission::Member {
					permissions: granted
						.iter()
						.fold(BTreeMap::new(), |mut permissions, row| {
							permissions
								.entry(row.permission_id)
								.or_insert_with(BTreeSet::new)
								.insert(row.scope_id);
							permissions
						}),
				},
				workspace_id,
			)
		}) {
		return Err(ErrorType::GrantExceedsPermissions);
	}

	let token = permissions::generate_service_account_token();
	let token_hash = hex::encode(Sha256::digest(&token));

	// `old` is the row as it was before the update (Postgres 18), so this hands
	// back the hash being replaced, whose cache entry has to go. The name is
	// for the email.
	let Some(old) = query!(
		r#"
		UPDATE
			service_account
		SET
			token_hash = $1
		WHERE
			id = $2 AND
			deleted IS NULL
		RETURNING
			name,
			old.token_hash;
		"#,
		&token_hash,
		service_account_id,
	)
	.fetch_optional(&mut **database)
	.await?
	else {
		return Err(ErrorType::ResourceDoesNotExist);
	};

	permissions::mark_token_stale(redis, &service_account_id, &old.token_hash).await?;

	let super_admin = query!(
		r#"
		SELECT
			workspace.name::TEXT AS "workspace_name!",
			"user".email::TEXT AS "email!",
			"user".first_name
		FROM
			workspace
		INNER JOIN
			"user"
		ON
			"user".id = workspace.super_admin_id
		WHERE
			workspace.id = $1;
		"#,
		workspace_id,
	)
	.fetch_one(&mut **database)
	.await?;

	// Cloud serves the dashboard on `app.`; self-hosted off the base domain.
	let base_domain = &state.config.server.base_domain;
	let dashboard_url = if cfg!(feature = "cloud") {
		format!("https://app.{base_domain}")
	} else {
		format!("https://{base_domain}")
	};

	state
		.worker
		.send_email(
			super_admin.email,
			ServiceAccountTokenRegeneratedEmail {
				first_name: super_admin.first_name,
				service_account_name: old.name,
				workspace_name: super_admin.workspace_name,
				regenerated_by: actor_data.actor.display_name(),
				service_account_url: format!(
					"{dashboard_url}/workspace/service-accounts/{service_account_id}"
				),
			},
		)
		.await
		.inspect_err(|err| {
			error!("Error enqueuing service account token regenerated email: `{err}`");
		})?;

	AppResponse::builder()
		.body(RegenerateServiceAccountTokenResponse { token })
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
