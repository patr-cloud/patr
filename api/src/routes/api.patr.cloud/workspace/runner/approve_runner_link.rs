use argon2::{Algorithm, Argon2, PasswordHasher, Version, password_hash::generate_salt};
use axum::http::StatusCode;
#[cfg(feature = "cloud")]
use cloudflare::{
	endpoints::workerskv::write_key,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
#[cfg(feature = "cloud")]
use models::cloudflare::kv::*;
use models::{api::workspace::runner::*, prelude::*};
use rustis::commands::StringCommands;
use time::OffsetDateTime;

use crate::{
	models::{
		permissions,
		redis::{RunnerApprovedSetupData, RunnerSetupDataEntry},
	},
	prelude::*,
};

pub async fn approve_runner_link(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ApproveRunnerLinkPath {
					workspace_id,
					user_code,
				},
				query: (),
				headers:
					ApproveRunnerLinkRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ApproveRunnerLinkRequestProcessed { runner_name },
			},
		database,
		redis,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, ApproveRunnerLinkRequest>,
) -> Result<AppResponse<ApproveRunnerLinkRequest>, ErrorType> {
	let key = redis::keys::runner_setup_data(workspace_id, &user_code);

	let Some(raw) = redis.get::<Option<String>>(&key).await? else {
		return Err(ErrorType::ResourceDoesNotExist);
	};
	let entry = serde_json::from_str::<RunnerSetupDataEntry>(&raw)?;

	if entry.approved.is_some() {
		// Already approved by someone (or this user in another tab). The CLI
		// will pick up the existing credentials on its next verify poll.
		return Err(ErrorType::ResourceAlreadyExists);
	}

	// Reject a taken name up front. The insert below is also guarded against
	// the unique violation, but creating the Cloudflare tunnel happens first
	// and is an external side effect the transaction rollback cannot undo — so
	// the common duplicate-name case must never get that far.
	let name_taken = query!(
		r#"
		SELECT
			id
		FROM
			runner
		WHERE
			workspace_id = $1 AND
			name = $2 AND
			deleted IS NULL;
		"#,
		workspace_id as _,
		runner_name.as_ref(),
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	if name_taken {
		return Err(ErrorType::ResourceAlreadyExists);
	}

	// Resolve permission UUIDs once up front. The workspace-wide list grants
	// `permission_type = 'exclude'` with no exclude rows = "every resource of
	// that type in this workspace".
	let runner_execute_permission_id = permissions::get_permission_id(
		&mut **database,
		Permission::Runner(RunnerPermission::Execute),
	)
	.await;
	let mut workspace_wide_permission_ids = Vec::new();
	for permission in [
		Permission::Deployment(DeploymentPermission::View),
		Permission::Database(DatabasePermission::View),
		Permission::StaticSite(StaticSitePermission::View),
		Permission::Volume(VolumePermission::View),
		Permission::ManagedURL(ManagedURLPermission::View),
		Permission::Domain(DomainPermission::View),
		Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::View),
		Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Pull),
		Permission::Secret(SecretPermission::View),
	] {
		workspace_wide_permission_ids
			.push(permissions::get_permission_id(&mut **database, permission).await);
	}

	// Runner resource row
	let runner_id = query!(
		r#"
		INSERT INTO
			resource(
				id,
				resource_type_id,
				owner_id,
				created
			)
		VALUES
			(
				GENERATE_RESOURCE_ID(),
				(SELECT id FROM resource_type WHERE name = 'runner'),
				$1,
				NOW()
			)
		RETURNING id AS "id: Uuid";
		"#,
		workspace_id as _,
	)
	.fetch_one(&mut **database)
	.await?
	.id;

	// Per-runner role. A role is itself a resource, so the resource row must be
	// created first to satisfy role(id, owner_id) -> resource(id, owner_id).
	let now = OffsetDateTime::now_utc();
	let role_id = query!(
		r#"
		INSERT INTO
			resource(
				id,
				resource_type_id,
				owner_id,
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
	.fetch_one(&mut **database)
	.await?
	.id;

	query!(
		r#"
		INSERT INTO
			role(
				id,
				owner_id,
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
		format!("runner-{runner_id}") as _,
		format!("Auto-generated role for runner '{runner_name}' service account") as _,
	)
	.execute(&mut **database)
	.await?;

	// Workspace-wide grants — `permission_type = 'exclude'` with no rows in
	// `role_resource_permissions_exclude` means "all resources allowed".
	for permission_id in &workspace_wide_permission_ids {
		query!(
			r#"
			INSERT INTO
				role_resource_permissions_type(
					role_id,
					permission_id,
					permission_type
				)
			VALUES
				($1, $2, 'exclude');
			"#,
			role_id as _,
			permission_id as _,
		)
		.execute(&mut **database)
		.await?;
	}

	// Runner::Execute scoped to just this runner
	query!(
		r#"
		INSERT INTO
			role_resource_permissions_type(
				role_id,
				permission_id,
				permission_type
			)
		VALUES
			($1, $2, 'include');
		"#,
		role_id as _,
		runner_execute_permission_id as _,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		INSERT INTO
			role_resource_permissions_include(
				role_id,
				permission_id,
				resource_id
			)
		VALUES
			($1, $2, $3);
		"#,
		role_id as _,
		runner_execute_permission_id as _,
		runner_id as _,
	)
	.execute(&mut **database)
	.await?;

	// Service account: resource row + service_account row
	let refresh_token = Uuid::new_v4();
	let token_hash = Argon2::new_with_secret(
		state.config.password_pepper.as_bytes(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.map_err(ErrorType::server_error)?
	.hash_password_with_salt(refresh_token.as_bytes(), &generate_salt())
	.map_err(ErrorType::server_error)?
	.to_string();

	let sa_id = query!(
		r#"
		INSERT INTO
			resource(
				id,
				resource_type_id,
				owner_id,
				created
			)
		VALUES
			(
				GENERATE_RESOURCE_ID(),
				(SELECT id FROM resource_type WHERE name = 'serviceAccount'),
				$1,
				NOW()
			)
		RETURNING id AS "id: Uuid";
		"#,
		workspace_id as _,
	)
	.fetch_one(&mut **database)
	.await?
	.id;

	// A service account is an identity holding exactly one non-rotating
	// credential keyed on its own ID.
	query!(
		r#"
		INSERT INTO
			identity(id, type)
		VALUES
			($1, 'service_account');
		"#,
		sa_id as _,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		INSERT INTO
			service_account(
				id,
				name,
				workspace_id,
				created,
				description,
				token_hash
			)
		VALUES
			($1, $2, $3, NOW(), $4, $5);
		"#,
		sa_id as _,
		format!("runner-{runner_id}"),
		workspace_id as _,
		Some(format!("Service account for runner '{runner_name}'")),
		&token_hash,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		INSERT INTO
			credential(credential_id, identity_id, type, created)
		VALUES
			($1, $1, 'service_account', NOW());
		"#,
		sa_id as _,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		INSERT INTO
			workspace_member(
				identity_id,
				workspace_id,
				role_id
			)
		VALUES
			($1, $2, $3);
		"#,
		sa_id as _,
		workspace_id as _,
		role_id as _,
	)
	.execute(&mut **database)
	.await?;

	// Cloudflare tunnel for the runner. Self-hosted has no Cloudflare account to
	// create one on, so the column is left empty there.
	cfg_if! {
		if #[cfg(feature = "cloud")] {
			let tunnel_id =
				utils::cloudflare::create_tunnel_with_config(runner_id, &state.config).await?;
		} else {
			let tunnel_id = String::new();
		}
	}

	// Runner row, now that the SA exists for the FK
	query!(
		r#"
		INSERT INTO
			runner(
				id,
				name,
				is_connected,
				workspace_id,
				cloudflare_tunnel_id,
				version,
				service_account_id
			)
		VALUES
			($1, $2, FALSE, $3, $4, '0.0.0', $5);
		"#,
		runner_id as _,
		runner_name.as_ref(),
		workspace_id as _,
		tunnel_id,
		sa_id as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		// Backstop for two approvals racing past the check above.
		sqlx::Error::Database(dbe) if dbe.is_unique_violation() => ErrorType::ResourceAlreadyExists,
		err => err.into(),
	})?;

	// Cloudflare KV registers the runner so the worker routes to it
	cfg_if! {
		if #[cfg(feature = "cloud")] {
			CloudflareClient::new(
				Credentials::UserAuthToken {
					token: state.config.cloudflare.api_key.clone(),
				},
				ClientConfig::default(),
				Environment::Custom(state.config.cloudflare.base_url.clone()),
			)?
			.request(&write_key::WriteKey {
				account_identifier: &state.config.cloudflare.account_id,
				namespace_identifier: &state.config.cloudflare.worker_namespace_id,
				key: &runner_id.to_string(),
				params: write_key::WriteKeyParams {
					expiration: None,
					expiration_ttl: None,
				},
				body: write_key::WriteKeyBody::Value(serde_json::to_vec(&InternalKVData::Runner)?),
			})
			.await?;
		} else {
			let _ = &state;
		}
	}

	// Mark the link approved in Redis. CLI's next verify poll picks this up and
	// one-shot deletes the entry.
	redis
		.setex(
			&key,
			constants::RUNNER_LINK_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			serde_json::to_string(&RunnerSetupDataEntry {
				approved: Some(RunnerApprovedSetupData {
					runner_id,
					workspace_id,
					token: format!("patrv1.{}.{}", refresh_token, sa_id),
				}),
				..entry
			})?,
		)
		.await?;

	AppResponse::builder()
		.body(ApproveRunnerLinkResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
