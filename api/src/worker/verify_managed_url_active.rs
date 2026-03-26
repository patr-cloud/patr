use std::time::Duration;

use apalis::prelude::*;
use apalis_cron::Tick;
use futures::TryStreamExt;

use crate::prelude::*;

/// The cron job that verifies managed URL active status every 2 hours.
///
/// For every managed URL in the database, construct the FQDN and make an HTTP
/// request to `https://{fqdn}/.well-known/patr/managed-url`. If the response
/// is 200, the URL is actively being served by Patr.
pub async fn verify_managed_url_active(_: Tick, data: Data<AppState>) -> Result<(), WorkerError> {
	println!("Verifying managed URL active status...");

	let client = reqwest::Client::new();

	query!(
		r#"
		SELECT
			managed_url.id,
			managed_url.sub_domain,
			managed_url.is_active,
			workspace_domain.name AS domain_name,
			workspace_domain.tld AS domain_tld
		FROM
			managed_url
		INNER JOIN
			workspace_domain
		ON
			managed_url.domain_id = workspace_domain.id
		WHERE
			managed_url.deleted IS NULL;
		"#,
	)
	.fetch(&data.database)
	.map_err(ErrorType::server_error)
	.try_for_each(async |row| {
		let fqdn = if row.sub_domain == "@" {
			format!("{}.{}", row.domain_name, row.domain_tld)
		} else {
			format!("{}.{}.{}", row.sub_domain, row.domain_name, row.domain_tld)
		};

		let is_active = client
			.get(format!("https://{}/.well-known/patr/managed-url", fqdn))
			.header("Cache-Control", "no-cache")
			.timeout(Duration::from_secs(10))
			.send()
			.await
			.map(|resp| resp.status().is_success())
			.unwrap_or(false);

		if is_active != row.is_active {
			query!(
				r#"
				UPDATE
					managed_url
				SET
					is_active = $2
				WHERE
					id = $1;
				"#,
				row.id as _,
				is_active,
			)
			.execute(&data.database)
			.await?;
		}

		Ok(())
	})
	.await
	.map_err(|err| WorkerStateError::InvalidState(err.to_string()).into())
}
