use apalis::prelude::*;
use apalis_cron::Tick;
use futures::TryStreamExt;
use hickory_resolver::{
	Resolver,
	config::ResolverConfig,
	net::runtime::TokioRuntimeProvider,
	proto::rr::RData,
};

use crate::prelude::*;

/// The cron job that re-verifies verified domains every 6 hours.
///
/// For every verified domain in the database, perform a DNS TXT lookup for
/// `_patr-verify.{domain}` and check if the value still matches the domain ID.
/// If it no longer does, mark the domain as unverified.
pub async fn reverify_verified_domains(_: Tick, data: Data<AppState>) -> Result<(), WorkerError> {
	println!("Re-verifying verified domains...");

	let resolver =
		Resolver::builder_with_config(ResolverConfig::default(), TokioRuntimeProvider::default())
			.build()
			.expect("failed to build DNS resolver");

	query!(
		r#"
		SELECT
			id AS "id: Uuid",
			name,
			tld
		FROM
			workspace_domain
		WHERE
			is_verified = TRUE AND
			deleted IS NULL;
		"#,
	)
	.fetch(&data.database)
	.map_err(ErrorType::server_error)
	.try_for_each(async |domain| {
		let verification_hostname = format!("_patr-verify.{}.{}", domain.name, domain.tld);
		let expected_value = domain.id.to_string();

		let verified = match resolver.txt_lookup(&verification_hostname).await {
			Ok(lookup) => lookup.answers().iter().any(|record| {
				let RData::TXT(txt) = &record.data else {
					return false;
				};
				txt.txt_data
					.iter()
					.any(|data| String::from_utf8_lossy(data) == expected_value)
			}),
			Err(_) => false,
		};

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
