use cloudflare::{
	endpoints::zone::{self, Status, Zone},
	framework::async_api::{ApiClient, Client},
};

use crate::{db, scheduler::Job, service, utils::validator};

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

pub async fn refresh_domain_tld_list() -> crate::Result<()> {
	let data =
		reqwest::get("https://data.iana.org/TLD/tlds-alpha-by-domain.txt")
			.await?
			.text()
			.await?;

	let tlds = data
		.split('\n')
		.map(String::from)
		.filter(|tld| {
			!tld.starts_with('#') && !tld.is_empty() && !tld.starts_with("XN--")
		})
		.collect::<Vec<String>>();

	validator::update_domain_tld_list(tlds).await;
	Ok(())
}

async fn verify_unverified_domains() -> crate::Result<()> {
	let config = super::CONFIG.get().unwrap();
	let mut connection = config.database.begin().await?;

	let settings = config.config.clone();

	let unverified_domains =
		db::get_all_unverified_domains(&mut connection).await?;

	let client = service::get_cloudflare_client(&config.config).await;

	let client = match client {
		Ok(client) => client,
		Err(err) => {
			log::error!("Cannot get cloudflare client: {}", err.get_error());
			return Ok(());
		}
	};

	for unverified_domain in unverified_domains {
		let zone = get_zone_for_domain(&client, &unverified_domain.name).await;

		if let Some(zone) = &zone {
			let response = client
				.request(&zone::ZoneDetails {
					identifier: &zone.id,
				})
				.await?;

			if let Status::Active = response.result.status {
				// Domain is now verified
				db::set_domain_as_verified(
					&mut connection,
					&unverified_domain.id,
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
					// TODO change this to notifier
					// mailer::send_domain_verified_mail(
					// 	config.config.clone(),
					// 	notification_email.unwrap(),
					// 	unverified_domain.name,
					// );
				}
				continue;
			}
			// else

			// Domain is still not verified. Initiate zone activation check
			let reqwest_client = reqwest::Client::new();
			let response = reqwest_client
				.put(format!(
				"https://api.cloudflare.com/client/v4/zones/{}/activation_check",
				response.result.id
			))
				.header(
					"Authorization",
					format!("Bearer {}", config.config.cloudflare.api_token),
				)
				.send()
				.await;
			if let Err(err) = response {
				log::error!("Cannot initiate zone activation check: {}", err);
			}
		} else {
			// add to cflr
			log::error!(
				"Domain `{}` was not added to cloudflare. Adding again.",
				unverified_domain.name
			);
			let response = client
				.request(&zone::CreateZone {
					params: zone::CreateZoneParams {
						account: zone::AccountId {
							id: &config.config.cloudflare.account_id,
						},
						name: &unverified_domain.name,
						jump_start: Some(false),
						zone_type: Some(zone::Type::Full),
					},
				})
				.await?;
			if !response.errors.is_empty() {
				log::error!(
					"Domain `{}` errored while adding to cloudflare: {:#?}",
					unverified_domain.name,
					response.errors
				);
				continue;
			}
		}
		let zone = zone.unwrap();

		let response = client
			.request(&zone::ZoneDetails {
				identifier: &zone.id,
			})
			.await?;

		if let Status::Active = response.result.status {
			let workspace_id =
				db::get_resource_by_id(&mut connection, &unverified_domain.id)
					.await?
					.unwrap()
					.owner_id;
			if unverified_domain.is_ns_internal() {
				// TODO: handle this unwrap
				let certificate_status = service::create_certificates(
					&workspace_id,
					&format!("certificate-{}", unverified_domain.id),
					&format!("tls-{}", unverified_domain.id),
					vec![unverified_domain.name.clone()],
					&settings,
				)
				.await;

				if let Err(error) = certificate_status {
					log::error!(
						"Domain `{}` errored while creating certificates: {}",
						unverified_domain.name,
						error.get_error()
					);
					continue;
				}
			} else {
				let managed_urls = db::get_all_managed_urls_for_domain(
					&mut connection,
					&unverified_domain.id,
				)
				.await?;

				for managed_url in managed_urls {
					let certificate_status = service::create_certificates(
						&workspace_id,
						&format!("certificate-{}", managed_url.id),
						&format!("tls-{}", managed_url.id),
						vec![format!(
							"{}.{}",
							managed_url.sub_domain, unverified_domain.name
						)],
						&settings,
					)
					.await;

					if let Err(error) = certificate_status {
						log::error!(
							"Domain `{}` errored while creating certificates: {}",
							unverified_domain.name,
							error.get_error()
						);
						continue;
					}
				}
			}

			// Domain is now verified
			db::set_domain_as_verified(&mut connection, &unverified_domain.id)
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
				// TODO change this to notifier
				// mailer::send_domain_verified_mail(
				// 	config.config.clone(),
				// 	notification_email.unwrap(),
				// 	unverified_domain.name,
				// );
			}
			continue;
		}
		// else

		// Domain is still not verified. Initiate zone activation check
		let reqwest_client = reqwest::Client::new();
		let response = reqwest_client
			.put(format!(
				"https://api.cloudflare.com/client/v4/zones/{}/activation_check",
				response.result.id
			))
			.header(
				"Authorization",
				format!("Bearer {}", config.config.cloudflare.api_token),
			)
			.send()
			.await;
		if let Err(err) = response {
			log::error!("Cannot initiate zone activation check: {}", err);
		}
	}

	Ok(())
}

async fn reverify_verified_domains() -> crate::Result<()> {
	let config = super::CONFIG.get().unwrap();
	let mut connection = config.database.begin().await?;

	let verified_domains =
		db::get_all_verified_domains(&mut connection).await?;

	let client = service::get_cloudflare_client(&config.config).await;

	let client = match client {
		Ok(client) => client,
		Err(err) => {
			log::error!("Cannot get cloudflare client: {}", err.get_error());
			return Ok(());
		}
	};

	for verified_domain in verified_domains {
		let zone = get_zone_for_domain(&client, &verified_domain.name).await;
		if let Some(zone) = zone {
			let response = client
				.request(&zone::ZoneDetails {
					identifier: &zone.id,
				})
				.await?;

			if let Status::Active = response.result.status {
				continue;
			}
			// Domain is now unverified
			db::set_domain_as_unverified(&mut connection, &verified_domain.id)
				.await?;
			let notification_email = db::get_notification_email_for_domain(
				&mut connection,
				&verified_domain.id,
			)
			.await?;
			if notification_email.is_none() {
				log::error!("Notification email for domain `{}` is None. You might have a dangling resource for the domain", verified_domain.name);
				continue;
			}
		// TODO change this to notifier
		// mailer::send_domain_unverified_mail(
		// 	config.config.clone(),
		// 	notification_email.unwrap(),
		// 	verified_domain.name,
		// );
		} else {
			// add to cflr
			log::error!(
				"Domain `{}` was not added to cloudflare. Adding again.",
				verified_domain.name
			);
			let response = client
				.request(&zone::CreateZone {
					params: zone::CreateZoneParams {
						account: zone::AccountId {
							id: &config.config.cloudflare.account_id,
						},
						name: &verified_domain.name,
						jump_start: Some(false),
						zone_type: Some(zone::Type::Full),
					},
				})
				.await?;
			if !response.errors.is_empty() {
				log::error!(
					"Domain `{}` errored while adding to cloudflare: {:#?}",
					verified_domain.name,
					response.errors
				);
				continue;
			}
		}
	}

	Ok(())
}

pub async fn get_zone_for_domain(
	client: &Client,
	domain: &str,
) -> Option<Zone> {
	client
		.request(&zone::ListZones {
			params: zone::ListZonesParams {
				name: Some(domain.to_string()),
				..Default::default()
			},
		})
		.await
		.ok()
		.map(|zones| zones.result.into_iter().next())
		.flatten()
}
