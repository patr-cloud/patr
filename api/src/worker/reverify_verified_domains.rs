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

pub async fn reverify_verified_domains(_: Tick, data: Data<AppState>) -> Result<(), WorkerError> {
	println!("Re-verifying verified domains...");

	query!(
		r#"
	    SELECT
	        id,
	        cloudflare_custom_hostname_id
	    FROM
	        workspace_domain
	    WHERE
	        is_verified = TRUE;
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
		.ssl
		.as_ref()
		.and_then(|ssl| ssl.status.as_deref())
		.map(|status| status == "active")
		.unwrap_or(false);

		if !verified {
			query!(
				r#"
				UPDATE
					workspace_domain
				SET
					is_verified = FALSE,
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
		// their domain has been unverified

		Ok(())
	})
	.await
	.map_err(|err| WorkerStateError::InvalidState(err.to_string()).into())
}
