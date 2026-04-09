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

/// The cron job that cleans up managed URLs whose FQDN custom hostname has
/// been inactive for more than 7 days. If a custom hostname stays inactive
/// for that long, the managed URLs and the custom hostname are removed.
pub async fn cleanup_inactive_managed_urls(
	_: Tick,
	data: Data<AppState>,
) -> Result<(), WorkerError> {
	println!("Cleaning up long-inactive managed URL FQDNs...");

	let inactive_fqdns = query!(
		r#"
		SELECT
			managed_url_custom_hostname.sub_domain,
			managed_url_custom_hostname.domain_id AS "domain_id: Uuid",
			managed_url_custom_hostname.cloudflare_custom_hostname_id,
			CONCAT(
				workspace_domain.name,
				'.',
				workspace_domain.tld
			) AS "domain_name!"
		FROM
			managed_url_custom_hostname
		INNER JOIN
			workspace_domain
		ON
			managed_url_custom_hostname.domain_id = workspace_domain.id
		WHERE
			managed_url_custom_hostname.is_active = FALSE AND
			managed_url_custom_hostname.last_verified IS NOT NULL AND
			managed_url_custom_hostname.last_verified < NOW() - INTERVAL '7 days';
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

	for fqdn in &inactive_fqdns {
		let fqdn_str = format!("{}.{}", fqdn.sub_domain, fqdn.domain_name);

		info!(
			"Cleaning up inactive FQDN `{}` (CF hostname: {})",
			fqdn_str, fqdn.cloudflare_custom_hostname_id
		);

		// Delete all managed URLs for this FQDN
		let managed_urls = query!(
			r#"
			SELECT
				managed_url.id AS "id: Uuid",
				deployment.runner AS "runner?: Uuid"
			FROM
				managed_url
			LEFT JOIN
				deployment
			ON
				managed_url.deployment_id = deployment.id
			WHERE
				managed_url.sub_domain = $1 AND
				managed_url.domain_id = $2 AND
				managed_url.deleted IS NULL;
			"#,
			&fqdn.sub_domain,
			fqdn.domain_id as _,
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

		// Delete the custom hostname row
		query!(
			r#"
			DELETE FROM
				managed_url_custom_hostname
			WHERE
				sub_domain = $1 AND
				domain_id = $2;
			"#,
			&fqdn.sub_domain,
			fqdn.domain_id as _,
		)
		.execute(&data.database)
		.await
		.map_err(|err| WorkerStateError::InvalidState(err.to_string()))?;

		// Delete the CF custom hostname
		match cf_client
			.request(&DeleteCustomHostname {
				zone_identifier: &data.config.cloudflare.primary_hosted_zone_id,
				custom_hostname_id: &fqdn.cloudflare_custom_hostname_id,
			})
			.await
		{
			Ok(_) => {}
			Err(ApiFailure::Error(status, _)) if status == reqwest::StatusCode::NOT_FOUND => {}
			Err(err) => {
				error!(
					"Failed to delete CF custom hostname {}: {}",
					fqdn.cloudflare_custom_hostname_id, err
				);
			}
		}

		// Sync KV
		let mut conn = data
			.database
			.acquire()
			.await
			.map_err(|err| WorkerStateError::InvalidState(err.to_string()))?;

		utils::cloudflare::sync_ingress_kv_for_fqdn(&fqdn_str, &mut conn, &data.config)
			.await
			.map_err(|err| WorkerStateError::InvalidState(err.to_string()))?;

		// Update runner configs
		for url in &managed_urls {
			if let Some(runner_id) = url.runner {
				utils::cloudflare::update_tunnel_config_for_runner(
					runner_id,
					&mut conn,
					&data.config,
				)
				.await
				.map_err(|err| WorkerStateError::InvalidState(err.to_string()))?;
			}
		}

		// TODO send an email to the workspace super-admin notifying them that
		// the managed URLs for this FQDN have been removed

		info!("Cleaned up inactive FQDN `{}`", fqdn_str);
	}

	Ok(())
}
