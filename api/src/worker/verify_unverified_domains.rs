use apalis::prelude::*;
use apalis_cron::Tick;
use cloudflare::{
	endpoints::zones::custom_hostnames::*,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
use futures::TryStreamExt;

use crate::prelude::*;

/// The cron job that verifies unverified domains every 2 hours.
///
/// For every unverified domain in the database, get it's Cloudflare custom
/// hostname ID, and hit the cloudflare endpoint to check if it's verified. If
/// it is, update the database to mark it as verified.
pub async fn verify_unverified_domains(_: Tick, data: Data<AppState>) -> Result<(), WorkerError> {
	println!("Verifying unverified domains...");

	query!(
		r#"
	    SELECT
	        id,
	        cloudflare_custom_hostname_id
	    FROM
	        workspace_domain
	    WHERE
	        is_verified = FALSE;
	    "#,
	)
	.fetch(&data.database)
	.map_err(ErrorType::server_error)
	.try_for_each(async |domain| {
		// Check with Cloudflare if the domain is verified
		let verified = CloudflareClient::new(
			Credentials::UserAuthToken {
				token: data.config.cloudflare.api_key.clone(),
			},
			ClientConfig::default(),
			Environment::Custom(data.config.cloudflare.base_url.clone()),
		)?
		.request(&EditCustomHostname {
			zone_identifier: &data.config.cloudflare.primary_hosted_zone_id,
			custom_hostname_id: &domain.cloudflare_custom_hostname_id,
			params: EditCustomHostnameParams {
				custom_metadata: None,
				custom_origin_server: None,
				custom_origin_sni: None,
				ssl: None,
			},
		})
		.await?
		.result
		.status == "active";

		if verified {
			query!(
				r#"
				UPDATE
					workspace_domain
				SET
					is_verified = TRUE,
					last_verified = NOW()
				WHERE
					id = $1;
				"#,
				domain.id as _,
			)
			.execute(&data.database)
			.await?;
		}

		// TODO send an email to the super-admin of the workspace notifying them that
		// their domain has been verified

		Ok(())
	})
	.await
	.map_err(|err| WorkerStateError::InvalidState(err.to_string()).into())
}
