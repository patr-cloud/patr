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
use futures::TryStreamExt;

use crate::prelude::*;

/// The cron job that verifies FQDN custom hostname status every 2 hours.
///
/// For every custom hostname in the database, check with Cloudflare whether
/// the hostname is active. Update the `is_active` flag accordingly.
///
/// Also acts as a reconciliation loop: if a CF custom hostname was deleted
/// externally (not found), recreate it and update the stored ID.
pub async fn verify_managed_url_active(_: Tick, data: Data<AppState>) -> Result<(), WorkerError> {
	println!("Verifying FQDN custom hostname status...");

	let cf_client = CloudflareClient::new(
		Credentials::UserAuthToken {
			token: data.config.cloudflare.api_key.clone(),
		},
		ClientConfig::default(),
		Environment::Custom(data.config.cloudflare.base_url.clone()),
	)
	.map_err(|err| WorkerStateError::InvalidState(err.to_string()))?;

	// Check status of all existing custom hostnames
	query!(
		r#"
		SELECT
			managed_url_custom_hostname.sub_domain,
			managed_url_custom_hostname.domain_id AS "domain_id: Uuid",
			managed_url_custom_hostname.cloudflare_custom_hostname_id,
			managed_url_custom_hostname.is_active,
			workspace_domain.name AS "domain_name",
			workspace_domain.tld AS "domain_tld"
		FROM
			managed_url_custom_hostname
		INNER JOIN
			workspace_domain
		ON
			managed_url_custom_hostname.domain_id = workspace_domain.id;
		"#,
	)
	.fetch(&data.database)
	.map_err(ErrorType::server_error)
	.try_for_each(async |row| {
		let result = cf_client
			.request(&EditCustomHostname {
				zone_identifier: &data.config.cloudflare.primary_hosted_zone_id,
				custom_hostname_id: &row.cloudflare_custom_hostname_id,
				params: EditCustomHostnameParams {
					custom_metadata: None,
					custom_origin_server: None,
					custom_origin_sni: None,
					ssl: None,
				},
			})
			.await;

		match result {
			Ok(response) => {
				let active = response.result.status == "active";

				if active != row.is_active {
					query!(
						r#"
						UPDATE
							managed_url_custom_hostname
						SET
							is_active = $3,
							last_verified = NOW()
						WHERE
							sub_domain = $1 AND
							domain_id = $2;
						"#,
						&row.sub_domain,
						row.domain_id as _,
						active,
					)
					.execute(&data.database)
					.await?;
				}
			}
			Err(ApiFailure::Error(status, _)) if status == reqwest::StatusCode::NOT_FOUND => {
				// CF hostname was deleted externally — recreate it
				let fqdn = if row.sub_domain == "@" {
					format!("{}.{}", row.domain_name, row.domain_tld)
				} else {
					format!("{}.{}.{}", row.sub_domain, row.domain_name, row.domain_tld)
				};

				info!("CF custom hostname not found for `{}`, recreating...", fqdn);

				let response = cf_client
					.request(&AddCustomHostname {
						zone_identifier: &data.config.cloudflare.primary_hosted_zone_id,
						params: AddCustomHostnameParams {
							hostname: fqdn.clone(),
							ssl: Some(CustomHostnameSsl {
								bundle_method: Some(CustomHostnameSslBundleMethod::Ubiquitous),
								certificate_authority: Some(
									CustomHostnameSslCertificateAuthority::LetsEncrypt,
								),
								type_: Some(CustomHostnameSslType::DV),
								method: Some(CustomHostnameSslMethod::Http),
								validation_records: None,
								settings: None,
								wildcard: None,
								status: None,
							}),
							custom_metadata: None,
						},
					})
					.await
					.inspect_err(|err| {
						error!(
							"Failed to recreate CF custom hostname for `{}`: {}",
							fqdn, err
						)
					})?;

				query!(
					r#"
					UPDATE
						managed_url_custom_hostname
					SET
						cloudflare_custom_hostname_id = $3,
						is_active = FALSE,
						last_verified = NOW()
					WHERE
						sub_domain = $1 AND
						domain_id = $2;
					"#,
					&row.sub_domain,
					row.domain_id as _,
					&response.result.id,
				)
				.execute(&data.database)
				.await?;

				info!(
					"Recreated CF custom hostname for `{}`: {}",
					fqdn, response.result.id
				);
			}
			Err(err) => {
				error!(
					"Failed to check CF custom hostname {}: {}",
					row.cloudflare_custom_hostname_id, err
				);
			}
		}

		Ok(())
	})
	.await
	.map_err(|err| WorkerStateError::InvalidState(err.to_string()).into())
}
