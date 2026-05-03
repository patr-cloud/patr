use axum::http::StatusCode;
use cloudflare::{
	endpoints::workerskv::delete_key,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
use models::{api::workspace::runner::*, prelude::*};
use rustis::commands::GenericCommands as _;

use crate::prelude::*;

pub async fn remove_runner_from_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteRunnerPath {
					workspace_id: _,
					runner_id,
				},
				query: (),
				headers:
					DeleteRunnerRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: DeleteRunnerRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, DeleteRunnerRequest>,
) -> Result<AppResponse<DeleteRunnerRequest>, ErrorType> {
	info!("Deleting runner `{}`", runner_id);

	// Capture the SA id before we drop the runner row.
	let sa_id = query!(
		r#"
		SELECT
			service_account_id AS "service_account_id: Uuid"
		FROM
			runner
		WHERE
			id = $1;
		"#,
		runner_id as _,
	)
	.fetch_one(&mut **database)
	.await?
	.service_account_id;

	// Per-runner role we auto-created in approve_runner_link, identified by
	// its name convention. Manually-attached roles (if any) are not deleted —
	// just unlinked when service_account_role rows go.
	let runner_role_name = format!("runner-{runner_id}");
	let role_row = query!(
		r#"
		SELECT
			id AS "id: Uuid"
		FROM
			role
		WHERE
			name = $1;
		"#,
		runner_role_name,
	)
	.fetch_optional(&mut **database)
	.await?;

	if let Some(role_row) = &role_row {
		query!(
			r#"
			DELETE FROM
				role_resource_permissions_include
			WHERE
				role_id = $1;
			"#,
			role_row.id as _,
		)
		.execute(&mut **database)
		.await?;

		query!(
			r#"
			DELETE FROM
				role_resource_permissions_exclude
			WHERE
				role_id = $1;
			"#,
			role_row.id as _,
		)
		.execute(&mut **database)
		.await?;

		query!(
			r#"
			DELETE FROM
				role_resource_permissions_type
			WHERE
				role_id = $1;
			"#,
			role_row.id as _,
		)
		.execute(&mut **database)
		.await?;
	}

	query!(
		r#"
		DELETE FROM
			service_account_role
		WHERE
			service_account_id = $1;
		"#,
		sa_id as _,
	)
	.execute(&mut **database)
	.await?;

	// Drop runner first to release its FK to service_account.
	query!(
		r#"
		DELETE FROM
			runner
		WHERE
			id = $1;
		"#,
		runner_id as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(dbe) if dbe.is_foreign_key_violation() => ErrorType::ResourceInUse,
		err => err.into(),
	})?;

	query!(
		r#"
		DELETE FROM
			service_account
		WHERE
			id = $1;
		"#,
		sa_id as _,
	)
	.execute(&mut **database)
	.await?;

	if let Some(role_row) = role_row {
		query!(
			r#"
			DELETE FROM
				role
			WHERE
				id = $1;
			"#,
			role_row.id as _,
		)
		.execute(&mut **database)
		.await?;
	}

	// Soft-delete the underlying resource rows so audit logs / FKs to
	// `resource` remain consistent (matches the existing pattern elsewhere).
	query!(
		r#"
		UPDATE
			resource
		SET
			deleted = NOW()
		WHERE
			id IN ($1, $2);
		"#,
		runner_id as _,
		sa_id as _,
	)
	.execute(&mut **database)
	.await?;

	// Invalidate the cached workspace-for-runner lookup
	redis
		.del(redis::keys::workspace_id_for_runner(&runner_id))
		.await?;

	CloudflareClient::new(
		Credentials::UserAuthToken {
			token: state.config.cloudflare.api_key.clone(),
		},
		ClientConfig::default(),
		Environment::Custom(state.config.cloudflare.base_url.clone()),
	)?
	.request(&delete_key::DeleteKey {
		account_identifier: &state.config.cloudflare.account_id,
		namespace_identifier: &state.config.cloudflare.worker_namespace_id,
		key: &runner_id.to_string(),
	})
	.await?;

	AppResponse::builder()
		.body(DeleteRunnerResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
