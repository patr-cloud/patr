use api_models::{self, utils::Uuid};

use crate::{db, scheduler::Job, service, utils::Error};

// Every 15 mins
pub(super) fn configure_all_unconfigued_managed_urls_job() -> Job {
	Job::new(
		String::from("Repatch all managed URLs"),
		"0 0/15 * * * *".parse().unwrap(),
		|| Box::pin(configure_all_unconfigured_managed_urls()),
	)
}

// Every day at 7 am
pub(super) fn reverify_all_configured_managed_urls_job() -> Job {
	Job::new(
		String::from("Reverify all managed URLs for external domain"),
		"0 0 7 * * *".parse().unwrap(),
		|| Box::pin(reverify_all_configured_managed_urls()),
	)
}

pub async fn configure_all_unconfigured_managed_urls() -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Re-patching all Managed URLs", request_id);
	let config = super::CONFIG.get().unwrap();
	let mut connection = config.database.begin().await?;
	let managed_urls =
		db::get_all_unconfigured_managed_urls(&mut connection).await?;

	for managed_url in managed_urls {
		let Ok(is_configured) = service::verify_managed_url_configuration(
			&mut connection,
			&managed_url.id,
			&config.config,
			&request_id,
		)
		.await
		.map_err(|err| {
			log::error!("Error verifying managed URL: {}", err);
			err
		}) else {
			continue;
		};

		if !is_configured {
			continue;
		}

		db::update_managed_url_configuration_status(
			&mut connection,
			&managed_url.id,
			is_configured,
		)
		.await?;
	}

	Ok(())
}

async fn reverify_all_configured_managed_urls() -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Re-patching all Managed URLs", request_id);
	let config = super::CONFIG.get().unwrap();
	let mut connection = config.database.begin().await?;
	let managed_urls =
		db::get_all_configured_managed_urls(&mut connection).await?;

	for managed_url in managed_urls {
		let Ok(is_configured) = service::verify_managed_url_configuration(
			&mut connection,
			&managed_url.id,
			&config.config,
			&request_id,
		)
		.await
		.map_err(|err| {
			log::error!("Error verifying managed URL: {}", err);
			err
		}) else {
			continue;
		};

		db::update_managed_url_configuration_status(
			&mut connection,
			&managed_url.id,
			is_configured,
		)
		.await?;
	}

	Ok(())
}
