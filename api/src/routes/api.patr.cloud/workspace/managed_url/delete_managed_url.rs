use axum::http::StatusCode;
use cloudflare::{
	endpoints::zones::custom_hostnames::*,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
		response::ApiFailure,
	},
};
use models::{
	api::workspace::{managed_url::*, runner::StreamRunnerDataForWorkspaceServerMsg},
	prelude::*,
};
use rustis::commands::PubSubCommands;

use crate::prelude::*;

/// The handler to delete a managed URL in a workspace. This will delete the
/// managed URL and remove it from the workspace. The managed URL must be owned
/// by the user and not already deleted.
pub async fn delete_managed_url(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteManagedURLPath {
					workspace_id,
					managed_url_id,
				},
				query: (),
				headers:
					DeleteManagedURLRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: DeleteManagedURLRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, DeleteManagedURLRequest>,
) -> Result<AppResponse<DeleteManagedURLRequest>, ErrorType> {
	info!("Deleting ManagedURL `{}`", managed_url_id);

	let managed_url = query!(
		r#"
		WITH deleted AS (
			DELETE FROM
				managed_url
			WHERE
				id = $1
			RETURNING
				sub_domain,
				domain_id,
				path,
				deployment_id
		)
		SELECT
			deleted.sub_domain,
			deleted.domain_id AS "domain_id: Uuid",
			CONCAT(
				workspace_domain.name,
				'.',
				workspace_domain.tld
			) AS "domain!",
			deleted.path,
			deployment.runner AS "connected_deployment_runner?: Uuid"
		FROM
			deleted
		INNER JOIN
			workspace_domain
		ON
			deleted.domain_id = workspace_domain.id
		LEFT JOIN
			deployment
		ON
			deployment.id = deleted.deployment_id;
		"#,
		managed_url_id as _,
	)
	.fetch_one(&mut **database)
	.await
	.map_err(|e| match e {
		sqlx::Error::Database(dbe) if dbe.is_foreign_key_violation() => ErrorType::ResourceInUse,
		err => ErrorType::server_error(err),
	})?;

	query!(
		r#"
		UPDATE
			resource
		SET
			deleted = NOW()
		WHERE
			id = $1;
		"#,
		managed_url_id as _,
	)
	.execute(&mut **database)
	.await?;

	utils::cloudflare::sync_ingress_kv_for_fqdn(
		&format!("{}.{}", managed_url.sub_domain, managed_url.domain),
		database,
		&state.config,
	)
	.await?;

	// Lock the custom hostname row to prevent race conditions with concurrent
	// create/delete operations on the same FQDN
	let locked_hostname = query!(
		r#"
		SELECT
			cloudflare_custom_hostname_id
		FROM
			managed_url_custom_hostname
		WHERE
			sub_domain = $1 AND
			domain_id = $2
		FOR UPDATE;
		"#,
		&managed_url.sub_domain,
		managed_url.domain_id as _,
	)
	.fetch_one(&mut **database)
	.await?;

	// Check if any managed URLs still use this FQDN
	let remaining = query!(
		r#"
		SELECT
			COUNT(*) AS "count!"
		FROM
			managed_url
		WHERE
			sub_domain = $1 AND
			domain_id = $2 AND
			deleted IS NULL;
		"#,
		&managed_url.sub_domain,
		managed_url.domain_id as _,
	)
	.fetch_one(&mut **database)
	.await?
	.count;

	if remaining == 0 {
		query!(
			r#"
			DELETE FROM
				managed_url_custom_hostname
			WHERE
				sub_domain = $1 AND
				domain_id = $2;
			"#,
			&managed_url.sub_domain,
			managed_url.domain_id as _,
		)
		.execute(&mut **database)
		.await?;

		let cf_client = CloudflareClient::new(
			Credentials::UserAuthToken {
				token: state.config.cloudflare.api_key.clone(),
			},
			ClientConfig::default(),
			Environment::Custom(state.config.cloudflare.base_url.clone()),
		)?;

		match cf_client
			.request(&DeleteCustomHostname {
				zone_identifier: &state.config.cloudflare.primary_hosted_zone_id,
				custom_hostname_id: &locked_hostname.cloudflare_custom_hostname_id,
			})
			.await
		{
			Ok(_) => {}
			Err(ApiFailure::Error(status, _)) if status == reqwest::StatusCode::NOT_FOUND => {}
			Err(err) => return Err(ErrorType::server_error(err)),
		}
	}

	// If the URL pointed at a deployment, tell that deployment's runner to
	// drop the corresponding Caddy config.
	if let Some(runner_id) = managed_url.connected_deployment_runner {
		redis
			.publish(
				format!("{}/runner/{}/stream", workspace_id, runner_id),
				serde_json::to_string(&StreamRunnerDataForWorkspaceServerMsg::ManagedUrlDeleted {
					id: managed_url_id,
				})
				.unwrap(),
			)
			.await?;
	}

	AppResponse::builder()
		.body(DeleteManagedURLResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
