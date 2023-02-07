use api_models::{self, utils::Uuid};
use chrono::Utc;
use cloudflare::{
	endpoints::zone::{self, Status},
	framework::{async_api::ApiClient, response::ApiFailure},
};
use eve_rs::AsError;
use sqlx::Connection;

use crate::{
	db,
	error,
	scheduler::Job,
	service,
	utils::{settings::Settings, validator, Error},
	Database,
};

// Every two hours
pub(super) fn verify_unverified_domains_job() -> Job {
	Job::new(
		String::from("Verify unverified domains"),
		"0 0 1/2 * * *".parse().unwrap(),
		|| Box::pin(verify_unverified_domains()),
	)
}

// Every day at 4 am
pub(super) fn reverify_verified_domains_job() -> Job {
	Job::new(
		String::from("Reverify verified domains"),
		"0 0 4 * * *".parse().unwrap(),
		|| Box::pin(reverify_verified_domains()),
	)
}

// Every day at 4 am
pub(super) fn refresh_domain_tld_list_job() -> Job {
	Job::new(
		String::from("Refresh domain TLD list"),
		"0 0 4 * * *".parse().unwrap(),
		|| Box::pin(refresh_domain_tld_list()),
	)
}

pub async fn refresh_domain_tld_list() -> Result<(), Error> {
	let mut connection = super::CONFIG.get().unwrap().database.begin().await?;
	let data =
		reqwest::get("https://publicsuffix.org/list/public_suffix_list.dat")
			.await?
			.text()
			.await?;

	let mut tlds = data
		.split('\n')
		.map(String::from)
		.filter(|tld| {
			!tld.starts_with('#') &&
				!tld.is_empty() && !tld.starts_with("XN--") &&
				!tld.starts_with("//") &&
				!tld.starts_with('!') &&
				tld.is_ascii()
		})
		.map(|item| item.to_lowercase().replace("*.", ""))
		.collect::<Vec<String>>();

	let god_user_id = db::get_god_user_id(&mut connection).await?;

	if god_user_id.is_none() {
		// No users have ever signed up. Do CI stuff here
		// Remove all TLDs and add them again to reset the order
		db::remove_from_domain_tld_list(&mut connection, &tlds).await?;
	}

	db::update_top_level_domain_list(&mut connection, &tlds).await?;

	let mut tld_list = validator::DOMAIN_TLD_LIST.write().await;
	tld_list.clear();
	tld_list.append(&mut tlds);
	drop(tld_list);

	connection.commit().await?;

	Ok(())
}

async fn verify_unverified_domains() -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Verifying unverified domains", request_id);
	let config = super::CONFIG.get().unwrap();
	let mut connection = config.database.acquire().await?;
	let settings = config.config.clone();
	let unverified_domains =
		db::get_all_unverified_domains(&mut connection).await?;
	let client = service::get_cloudflare_client(&config.config).await?;
	for (unverified_domain, zone_identifier) in unverified_domains {
		let domain_created_time =
			db::get_resource_by_id(&mut connection, &unverified_domain.id)
				.await?
				.status(500)? // resource will be there for a domain
				.created;
		let mut connection = connection.begin().await?;
		let workspace_id =
			db::get_resource_by_id(&mut connection, &unverified_domain.id)
				.await?
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?
				.owner_id;
		if let (Some(zone_identifier), true) =
			(zone_identifier, unverified_domain.is_ns_internal())
		{
			let zone_active = match client
				.request(&zone::ZoneDetails {
					identifier: &zone_identifier,
				})
				.await
			{
				Ok(response) => {
					matches!(response.result.status, Status::Active)
				}
				Err(ApiFailure::Error(status_code, _))
					if status_code == 400 =>
				{
					// The given domain does not exist in cloudflare. Something
					// is wrong here
					log::error!(
						"Domain `{}` does not exist in cloudflare",
						unverified_domain.name
					);
					false
				}
				Err(err) => {
					log::error!(
						"Unable to get domain `{}` from cloudflare: {}",
						unverified_domain.name,
						err
					);
					continue;
				}
			};
			if zone_active {
				// Domain is now verified
				db::update_workspace_domain_status(
					&mut connection,
					&unverified_domain.id,
					true,
					&Utc::now(),
				)
				.await?;
				let notification_email = db::get_notification_email_for_domain(
					&mut connection,
					&unverified_domain.id,
				)
				.await?;
				if notification_email.is_none() {
					log::error!(
						"Notification email for domain `{}` is None. {}",
						unverified_domain.name,
						"You might have a dangling resource for the domain"
					);
				} else {
					service::domain_verification_email(
						&mut connection,
						&unverified_domain.name,
						&workspace_id,
						&unverified_domain.id,
					)
					.await?
				}
				connection.commit().await?;
			} else {
				if let Some(last_verified) = unverified_domain.last_unverified {
					let last_unverified =
						Utc::now().signed_duration_since(last_verified);
					let last_unverified_days = last_unverified.num_days();
					if last_unverified_days > 5 {
						delete_unverified_domain(
							&mut connection,
							&workspace_id,
							&unverified_domain.id,
							true,
							&settings,
							&request_id,
						)
						.await?
					} else {
						continue;
					}
				} else {
					let domain_created =
						Utc::now().signed_duration_since(domain_created_time);
					let domain_created_days = domain_created.num_days() as u64;
					if domain_created_days > 15 {
						delete_unverified_domain(
							&mut connection,
							&workspace_id,
							&unverified_domain.id,
							true,
							&settings,
							&request_id,
						)
						.await?
					} else if domain_created_days > 12 &&
						domain_created_days <= 15
					{
						service::send_domain_verify_reminder_email(
							&mut connection,
							&workspace_id,
							&unverified_domain.name,
							true,
							&unverified_domain.id,
							15 - domain_created_days,
						)
						.await?
					} else {
						continue;
					}
				};
				connection.commit().await?;
			}
		} else {
			let verified = service::verify_external_domain(
				&unverified_domain.name,
				&unverified_domain.id,
				&request_id,
			)
			.await?;
			if verified {
				// domain is initially unverified
				// now it got verified
				db::update_workspace_domain_status(
					&mut connection,
					&unverified_domain.id,
					true,
					&Utc::now(),
				)
				.await?;
				service::domain_verification_email(
					&mut connection,
					&unverified_domain.name,
					&workspace_id,
					&unverified_domain.id,
				)
				.await?;
				connection.commit().await?;
			} else {
				log::error!(
					"Could not verify domain `{}`",
					unverified_domain.name
				);

				if let Some(last_unverified) = unverified_domain.last_unverified
				{
					let last_unverified =
						Utc::now().signed_duration_since(last_unverified);
					let last_unverified_days = last_unverified.num_days();
					if last_unverified_days > 5 {
						delete_unverified_domain(
							&mut connection,
							&workspace_id,
							&unverified_domain.id,
							false,
							&settings,
							&request_id,
						)
						.await?;
					} else {
						continue;
					}
				} else {
					let domain_created =
						Utc::now().signed_duration_since(domain_created_time);
					let domain_created_days = domain_created.num_days() as u64;
					if domain_created_days > 15 {
						delete_unverified_domain(
							&mut connection,
							&workspace_id,
							&unverified_domain.id,
							false,
							&settings,
							&request_id,
						)
						.await?;
					} else if domain_created_days > 12 &&
						domain_created_days <= 15
					{
						service::send_domain_verify_reminder_email(
							&mut connection,
							&workspace_id,
							&unverified_domain.name,
							true,
							&unverified_domain.id,
							15 - domain_created_days,
						)
						.await?
					}
					{
						continue;
					}
				}
				// todo: need to send an email when deleting domain resource
				connection.commit().await?;
			}
		}
	}
	Ok(())
}

async fn reverify_verified_domains() -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Re-verifying verified domains", request_id);
	let config = super::CONFIG.get().unwrap();
	let mut connection = config.database.acquire().await?;
	let verified_domains =
		db::get_all_verified_domains(&mut connection).await?;

	let client = service::get_cloudflare_client(&config.config).await?;

	for (verified_domain, zone_identifier) in verified_domains {
		let mut connection = config.database.begin().await?;
		let workspace_id =
			db::get_resource_by_id(&mut connection, &verified_domain.id)
				.await?
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?
				.owner_id;

		if let (Some(zone_identifier), true) =
			(zone_identifier, verified_domain.is_ns_internal())
		{
			// internal domain, so use cloudflare to verify it
			let response = match client
				.request(&zone::ZoneDetails {
					identifier: &zone_identifier,
				})
				.await
			{
				Ok(response) => response,
				Err(ApiFailure::Error(status_code, _))
					if status_code == 400 =>
				{
					// The given domain does not exist in cloudflare. Something
					// is wrong here
					log::error!(
						"Domain `{}` does not exist in cloudflare",
						verified_domain.name
					);
					continue;
				}
				Err(err) => {
					log::error!(
						"Unable to get domain `{}` from cloudflare: {}",
						verified_domain.name,
						err
					);
					continue;
				}
			};

			if let Status::Active = response.result.status {
				continue;
			}
			// Domain is now unverified
			db::update_workspace_domain_status(
				&mut connection,
				&verified_domain.id,
				false,
				&Utc::now(),
			)
			.await?;
			service::domain_unverified_email(
				&mut connection,
				&verified_domain.name,
				&workspace_id,
				&verified_domain.id,
				true,
				&5, //deadline limit
			)
			.await?
		} else {
			// external domain, so check txt records
			let verified = service::verify_external_domain(
				&verified_domain.name,
				&verified_domain.id,
				&request_id,
			)
			.await?;
			if !verified {
				log::error!(
					"Could not verify domain `{}`",
					verified_domain.name
				);

				db::update_workspace_domain_status(
					&mut connection,
					&verified_domain.id,
					false,
					&Utc::now(),
				)
				.await?;

				service::domain_unverified_email(
					&mut connection,
					&verified_domain.name,
					&workspace_id,
					&verified_domain.id,
					false,
					&5, //deadline limit
				)
				.await?
			}
		}

		// commit the changes made inside this transaction
		connection.commit().await?;
	}

	Ok(())
}

async fn delete_unverified_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	domain_id: &Uuid,
	is_internal: bool,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	// Delete all managed url before deleting the domain
	let managed_urls =
		db::get_all_managed_urls_for_domain(connection, domain_id).await?;
	for managed_url in managed_urls {
		service::delete_managed_url(
			connection,
			workspace_id,
			&managed_url.id,
			config,
			request_id,
		)
		.await?;
	}

	if is_internal {
		// Delete all the dns record before deleting the domain
		let dns_records =
			db::get_dns_records_by_domain_id(connection, domain_id).await?;
		for dns_record in dns_records {
			service::delete_patr_domain_dns_record(
				connection,
				domain_id,
				&dns_record.id,
				config,
				request_id,
			)
			.await?;
		}
	}
	// Delete the domain
	service::delete_domain_in_workspace(
		connection,
		workspace_id,
		domain_id,
		config,
		request_id,
	)
	.await?;

	Ok(())
}
