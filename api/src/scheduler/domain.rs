use api_models::{
	models::workspace::infrastructure::managed_urls::{
		ManagedUrl,
		ManagedUrlType,
	},
	utils::Uuid,
};
use chrono::Utc;
use cloudflare::{
	endpoints::zone::{self, Status},
	framework::{async_api::ApiClient, response::ApiFailure},
};
use eve_rs::AsError;
use sqlx::Connection;

use crate::{
	db::{self, ManagedUrlType as DbManagedUrlType},
	error,
	scheduler::Job,
	service,
	utils::{validator, Error},
};

// Every two hours
pub(super) fn verify_unverified_domains_job() -> Job {
	Job::new(
		String::from("Verify unverified domains"),
		"0 0 1/2 * * *".parse().unwrap(),
		|| Box::pin(verify_unverified_domains()),
	)
}

// Every 15 mins
pub(super) fn repatch_all_managed_urls_job() -> Job {
	Job::new(
		String::from("Repatch all managed URLs"),
		"0 0/15 * * * *".parse().unwrap(),
		|| Box::pin(repatch_all_managed_urls()),
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
			let response = match client
				.request(&zone::ZoneDetails {
					identifier: &zone_identifier,
				})
				.await
			{
				Ok(response) => response,
				Err(ApiFailure::Error(status_code, _))
					if status_code == 404 =>
				{
					// The given domain does not exist in cloudflare. Something
					// is wrong here
					log::error!(
						"Domain `{}` does not exist in cloudflare",
						unverified_domain.name
					);
					continue;
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
			if let Status::Active = response.result.status {
				service::create_certificates(
					&workspace_id,
					&format!("certificate-{}", unverified_domain.id),
					&format!("tls-{}", unverified_domain.id),
					vec![
						format!("*.{}", unverified_domain.name),
						unverified_domain.name.clone(),
					],
					true,
					&settings,
					&request_id,
				)
				.await?;
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
						true,
						true,
					)
					.await?
				}
				connection.commit().await?;
			} else {
				let last_unverified = Utc::now()
					.signed_duration_since(unverified_domain.last_unverified);
				let last_unverified_days = last_unverified.num_days();
				if last_unverified_days > 7 {
					// Delete all managed url before deleting the domain
					let managed_urls = db::get_all_managed_urls_for_domain(
						&mut connection,
						&unverified_domain.id,
					)
					.await?;
					for managed_url in managed_urls {
						service::delete_managed_url(
							&mut connection,
							&workspace_id,
							&managed_url.id,
							&settings,
							&request_id,
						)
						.await?;
					}
					// Delete all the dns record before deleting the domain
					let dns_records = db::get_dns_records_by_domain_id(
						&mut connection,
						&unverified_domain.id,
					)
					.await?;
					for dns_record in dns_records {
						service::delete_patr_domain_dns_record(
							&mut connection,
							&unverified_domain.id,
							&dns_record.id,
							&settings,
							&request_id,
						)
						.await?;
					}
					// Delete the domain
					service::delete_domain_in_workspace(
						&mut connection,
						&workspace_id,
						&unverified_domain.id,
						&settings,
						&request_id,
					)
					.await?;
					// Delete the certificate for the domain
					service::delete_certificates_for_domain(
						&workspace_id,
						&format!("certificate-{}", unverified_domain.id),
						&format!("tls-{}", unverified_domain.id),
						&settings,
						&request_id,
					)
					.await?;
					connection.commit().await?;
				} else {
					continue;
				}
			}
		} else {
			let response = service::verify_external_domain(
				&mut connection,
				&workspace_id,
				&unverified_domain.name,
				&unverified_domain.id,
				&request_id,
			)
			.await?;
			if !response {
				log::error!(
					"Could not verify domain `{}`",
					unverified_domain.name
				);

				// Sending mail
				service::domain_verification_email(
					&mut connection,
					&unverified_domain.name,
					&workspace_id,
					&unverified_domain.id,
					false,
					false,
				)
				.await?;

				let last_unverified = Utc::now()
					.signed_duration_since(unverified_domain.last_unverified);
				let last_unverified_days = last_unverified.num_days();
				if last_unverified_days > 7 {
					// Delete all managed url before deleting the domain
					let managed_urls = db::get_all_managed_urls_for_domain(
						&mut connection,
						&unverified_domain.id,
					)
					.await?;
					for managed_url in managed_urls {
						service::delete_managed_url(
							&mut connection,
							&workspace_id,
							&managed_url.id,
							&settings,
							&request_id,
						)
						.await?;
					}
					// Delete the domain
					service::delete_domain_in_workspace(
						&mut connection,
						&workspace_id,
						&unverified_domain.id,
						&settings,
						&request_id,
					)
					.await?;
				}
				connection.commit().await?;
			}
		}
	}
	Ok(())
}

async fn repatch_all_managed_urls() -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Re-patching all Managed URLs", request_id);
	let config = super::CONFIG.get().unwrap();
	let mut connection = config.database.begin().await?;
	let managed_urls =
		db::get_all_unconfigured_managed_urls(&mut connection).await?;

	for managed_url in managed_urls {
		let is_configured = service::verify_managed_url_configuration(
			&mut connection,
			&managed_url.id,
			&config.config,
			&request_id,
		)
		.await?;

		if !is_configured {
			continue;
		}

		let domain = db::get_workspace_domain_by_id(
			&mut connection,
			&managed_url.domain_id,
		)
		.await?
		.status(500)?;

		if domain.is_ns_external() {
			// External domain
			// Create certificate for the domain
			let secret_name = if managed_url.sub_domain == "@" {
				format!("tls-{}", managed_url.domain_id)
			} else {
				format!(
					"tls-{}-{}",
					managed_url.sub_domain, managed_url.domain_id
				)
			};
			service::create_certificates(
				&managed_url.workspace_id,
				&if managed_url.sub_domain == "@" {
					format!("certificate-{}", managed_url.domain_id)
				} else {
					format!(
						"certificate-{}-{}",
						managed_url.sub_domain, managed_url.domain_id
					)
				},
				&secret_name,
				vec![
					if managed_url.sub_domain == "@" {
						domain.name.to_string()
					} else {
						format!("{}.{}", managed_url.sub_domain, domain.name)
					},
				],
				false,
				&config.config,
				&request_id,
			)
			.await?;

			let cert_exists = service::is_kubernetes_certificate_secret_exists(
				&managed_url.workspace_id,
				&secret_name,
				&config.config,
				&request_id,
			)
			.await?;

			if cert_exists {
				db::update_managed_url_configuration_status(
					&mut connection,
					&managed_url.id,
					true,
				)
				.await?;

				service::update_kubernetes_managed_url(
					&managed_url.workspace_id,
					&ManagedUrl {
						id: managed_url.id,
						sub_domain: managed_url.sub_domain,
						domain_id: managed_url.domain_id,
						path: managed_url.path,
						url_type: match managed_url.url_type {
							DbManagedUrlType::ProxyToDeployment => {
								ManagedUrlType::ProxyDeployment {
									deployment_id: managed_url
										.deployment_id
										.status(500)?,
									port: managed_url.port.status(500)? as u16,
								}
							}
							DbManagedUrlType::ProxyToStaticSite => {
								ManagedUrlType::ProxyStaticSite {
									static_site_id: managed_url
										.static_site_id
										.status(500)?,
								}
							}
							DbManagedUrlType::ProxyUrl => {
								ManagedUrlType::ProxyUrl {
									url: managed_url.url.status(500)?,
								}
							}
							DbManagedUrlType::Redirect => {
								ManagedUrlType::Redirect {
									url: managed_url.url.status(500)?,
								}
							}
						},
						is_configured: true,
					},
					&config.config,
					&request_id,
				)
				.await?;
			}
		} else {
			let cert_exists = service::is_kubernetes_certificate_secret_exists(
				&managed_url.workspace_id,
				&format!("tls-{}", managed_url.domain_id),
				&config.config,
				&request_id,
			)
			.await?;

			if cert_exists {
				db::update_managed_url_configuration_status(
					&mut connection,
					&managed_url.id,
					true,
				)
				.await?;

				service::update_kubernetes_managed_url(
					&managed_url.workspace_id,
					&ManagedUrl {
						id: managed_url.id,
						sub_domain: managed_url.sub_domain,
						domain_id: managed_url.domain_id,
						path: managed_url.path,
						url_type: match managed_url.url_type {
							DbManagedUrlType::ProxyToDeployment => {
								ManagedUrlType::ProxyDeployment {
									deployment_id: managed_url
										.deployment_id
										.status(500)?,
									port: managed_url.port.status(500)? as u16,
								}
							}
							DbManagedUrlType::ProxyToStaticSite => {
								ManagedUrlType::ProxyStaticSite {
									static_site_id: managed_url
										.static_site_id
										.status(500)?,
								}
							}
							DbManagedUrlType::ProxyUrl => {
								ManagedUrlType::ProxyUrl {
									url: managed_url.url.status(500)?,
								}
							}
							DbManagedUrlType::Redirect => {
								ManagedUrlType::Redirect {
									url: managed_url.url.status(500)?,
								}
							}
						},
						is_configured: true,
					},
					&config.config,
					&request_id,
				)
				.await?;
			}
		}
	}

	Ok(())
}

async fn reverify_verified_domains() -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Re-verifying verified domains", request_id);
	let config = super::CONFIG.get().unwrap();
	let mut connection = config.database.begin().await?;

	let verified_domains =
		db::get_all_verified_domains(&mut connection).await?;

	let client = service::get_cloudflare_client(&config.config).await?;

	for (verified_domain, zone_identifier) in verified_domains {
		// getting workspace_id
		let workspace_id =
			db::get_resource_by_id(&mut connection, &verified_domain.id)
				.await?
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?
				.owner_id;

		let zone_identifier = if let Some(zone_identifier) = zone_identifier {
			zone_identifier
		} else {
			// TODO delete the domain altogether or add to cloudflare?
			continue;
		};
		let response = match client
			.request(&zone::ZoneDetails {
				identifier: &zone_identifier,
			})
			.await
		{
			Ok(response) => response,
			Err(ApiFailure::Error(status_code, _)) if status_code == 404 => {
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

		let notification_email = db::get_notification_email_for_domain(
			&mut connection,
			&verified_domain.id,
		)
		.await?;
		if notification_email.is_none() {
			log::error!("Notification email for domain `{}` is None. You might have a dangling resource for the domain", verified_domain.name);
			continue;
		} else {
			log::trace!(
				"domain: {} with id: {} is now unverfied",
				verified_domain.name,
				verified_domain.id
			);
			service::domain_verification_email(
				&mut connection,
				&verified_domain.name,
				&workspace_id,
				&verified_domain.id,
				true,
				false,
			)
			.await?
		}
		// TODO delete certificates and managed urls after 3 days
	}

	Ok(())
}
