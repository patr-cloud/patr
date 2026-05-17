use apalis::prelude::*;
use apalis_cron::Tick;
use cloudflare::{
	endpoints::zones::custom_hostnames::*,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
		response::ApiFailure,
	},
};

use crate::prelude::*;

/// The cron job that cleans up domains that have been unverified for more than
/// 7 days. This includes domains that were previously verified and became
/// unverified, as well as domains that were never verified after being added.
///
/// For each qualifying domain, all managed URLs, custom hostnames, and the
/// domain itself are deleted.
pub async fn cleanup_unverified_domains(_: Tick, data: Data<AppState>) -> Result<(), WorkerError> {
	println!("Cleaning up long-unverified domains...");

	// Previously-verified domains that have been unverified for 7+ days
	let expired_domains = query!(
		r#"
		SELECT
			workspace_domain.id AS "id: Uuid",
			workspace_domain.name,
			workspace_domain.tld,
			workspace_domain.workspace_id AS "workspace_id: Uuid"
		FROM
			workspace_domain
		INNER JOIN
			resource
		ON
			workspace_domain.id = resource.id
		WHERE
			workspace_domain.is_verified = FALSE AND
			workspace_domain.deleted IS NULL AND
			(
				(
					workspace_domain.last_verified IS NOT NULL AND
					workspace_domain.last_verified < NOW() - INTERVAL '7 days'
				) OR (
					workspace_domain.last_verified IS NULL AND
					resource.created < NOW() - INTERVAL '7 days'
				)
			);
		"#,
	)
	.fetch_all(&data.database)
	.await
	.map_err(|err| WorkerStateError::InvalidState(err.to_string()))?;

	let cf_client = CloudflareClient::new(
		Credentials::UserAuthToken {
			token: data.config.cloudflare.api_key.clone(),
		},
		ClientConfig::default(),
		Environment::Custom(data.config.cloudflare.base_url.clone()),
	)
	.map_err(|err| WorkerStateError::InvalidState(err.to_string()))?;

	for domain in &expired_domains {
		let domain_name = format!("{}.{}", domain.name, domain.tld);

		info!(
			"Cleaning up unverified domain `{}` (ID: {})",
			domain_name, domain.id
		);

		// Delete all managed URLs for this domain
		let managed_urls = query!(
			r#"
			SELECT
				managed_url.id AS "id: Uuid",
				managed_url.sub_domain,
				managed_url.deployment_id AS "deployment_id: Uuid",
				deployment.runner AS "runner?: Uuid"
			FROM
				managed_url
			LEFT JOIN
				deployment
			ON
				managed_url.deployment_id = deployment.id
			WHERE
				managed_url.domain_id = $1 AND
				managed_url.deleted IS NULL;
			"#,
			domain.id as _,
		)
		.fetch_all(&data.database)
		.await
		.map_err(|err| WorkerStateError::InvalidState(err.to_string()))?;

		for url in &managed_urls {
			query!(
				r#"
				DELETE FROM
					managed_url
				WHERE
					id = $1;
				"#,
				url.id as _,
			)
			.execute(&data.database)
			.await
			.map_err(|err| WorkerStateError::InvalidState(err.to_string()))?;

			query!(
				r#"
				UPDATE
					resource
				SET
					deleted = NOW()
				WHERE
					id = $1;
				"#,
				url.id as _,
			)
			.execute(&data.database)
			.await
			.map_err(|err| WorkerStateError::InvalidState(err.to_string()))?;
		}

		// Delete custom hostnames for this domain
		let custom_hostnames = query!(
			r#"
			DELETE FROM
				managed_url_custom_hostname
			WHERE
				domain_id = $1
			RETURNING
				cloudflare_custom_hostname_id;
			"#,
			domain.id as _,
		)
		.fetch_all(&data.database)
		.await
		.map_err(|err| WorkerStateError::InvalidState(err.to_string()))?;

		for hostname in &custom_hostnames {
			match cf_client
				.request(&DeleteCustomHostname {
					zone_identifier: &data.config.cloudflare.primary_hosted_zone_id,
					custom_hostname_id: &hostname.cloudflare_custom_hostname_id,
				})
				.await
			{
				Ok(_) => {}
				Err(ApiFailure::Error(status, _)) if status == reqwest::StatusCode::NOT_FOUND => {}
				Err(err) => {
					error!(
						"Failed to delete CF custom hostname {}: {}",
						hostname.cloudflare_custom_hostname_id, err
					);
				}
			}
		}

		// Sync KV for each unique FQDN that was removed
		for url in &managed_urls {
			let fqdn = format!("{}.{}", url.sub_domain, domain_name);
			// Pool doesn't implement DerefMut for DatabaseConnection, so we
			// need a connection from the pool
			let mut conn = data
				.database
				.acquire()
				.await
				.map_err(|err| WorkerStateError::InvalidState(err.to_string()))?;
			utils::cloudflare::sync_ingress_kv_for_fqdn(&fqdn, &mut conn, &data.config)
				.await
				.map_err(|err| WorkerStateError::InvalidState(err.to_string()))?;
		}

		query!(
			r#"
			DELETE FROM
				workspace_domain
			WHERE
				id = $1;
			"#,
			domain.id as _,
		)
		.execute(&data.database)
		.await
		.map_err(|err| WorkerStateError::InvalidState(err.to_string()))?;

		query!(
			r#"
			UPDATE
				resource
			SET
				deleted = NOW()
			WHERE
				id = $1;
			"#,
			domain.id as _,
		)
		.execute(&data.database)
		.await
		.map_err(|err| WorkerStateError::InvalidState(err.to_string()))?;

		// TODO send an email to the workspace super-admin notifying them that
		// the domain and its managed URLs have been removed

		info!("Cleaned up unverified domain `{}`", domain_name);
	}

	Ok(())
}
