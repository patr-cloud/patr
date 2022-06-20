use api_models::{
	models::workspace::infrastructure::managed_urls::{
		ManagedUrl,
		ManagedUrlType,
	},
	utils::Uuid,
};
use cloudflare::{
	endpoints::zone::{self, Status},
	framework::async_api::ApiClient,
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

// Every two hours
pub(super) fn repatch_all_managed_urls_job() -> Job {
	Job::new(
		String::from("Repatch all managed URLs"),
		"0 0 1/2 * * *".parse().unwrap(),
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

// Every 2 hour
pub(super) fn verify_transferred_domain_to_patr_job() -> Job {
	Job::new(
		String::from("Verify transferred domain to Patr"),
		"0 0 1/2 * * *".parse().unwrap(),
		|| Box::pin(verify_transferred_domain_to_patr()),
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
	let unused_tlds = db::get_all_unused_domain_tlds(&mut connection).await?;
	let depreciated_tlds = unused_tlds
		.into_iter()
		.filter(|tld| !tlds.contains(tld))
		.collect::<Vec<_>>();

	let god_user_id = db::get_god_user_id(&mut connection).await?;

	if god_user_id.is_none() {
		// No users have ever signed up. Do CI stuff here
		// Remove all TLDs and add them again to reset the order
		db::remove_from_domain_tld_list(&mut connection, &tlds).await?;
	}

	db::update_domain_tld_list(&mut connection, &tlds).await?;
	db::remove_from_domain_tld_list(&mut connection, &depreciated_tlds).await?;

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
			let response = client
				.request(&zone::ZoneDetails {
					identifier: &zone_identifier,
				})
				.await?;

			if let Status::Active = response.result.status {
				// Create certs below
			} else {
				continue;
			}

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

			connection.commit().await?;
		} else {
			let response = service::verify_external_domain(
				&mut connection,
				&workspace_id,
				&unverified_domain.name,
				&unverified_domain.id,
				&settings,
				&request_id,
			)
			.await?;

			if !response {
				log::error!(
					"Could not verify domain `{}`",
					unverified_domain.name
				);
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
	let managed_urls = db::get_all_managed_urls(&mut connection).await?;

	for managed_url in managed_urls {
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
					DbManagedUrlType::ProxyUrl => ManagedUrlType::ProxyUrl {
						url: managed_url.url.status(500)?,
					},
					DbManagedUrlType::Redirect => ManagedUrlType::Redirect {
						url: managed_url.url.status(500)?,
					},
				},
			},
			&config.config,
			&request_id,
		)
		.await?;
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

	let client = service::get_cloudflare_client(&config.config).await;

	let client = match client {
		Ok(client) => client,
		Err(err) => {
			log::error!("Cannot get cloudflare client: {}", err.get_error());
			return Ok(());
		}
	};

	for (verified_domain, zone_identifier) in verified_domains {
		let zone_identifier = if let Some(zone_identifier) = zone_identifier {
			zone_identifier
		} else {
			// TODO delete the domain altogether or add to cloudflare?
			continue;
		};
		let response = client
			.request(&zone::ZoneDetails {
				identifier: &zone_identifier,
			})
			.await?;

		if let Status::Active = response.result.status {
			continue;
		}
		// Domain is now unverified
		db::update_workspace_domain_status(
			&mut connection,
			&verified_domain.id,
			false,
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
			// TODO change this to notifier
			// mailer::send_domain_unverified_mail(
			// 	config.config.clone(),
			// 	notification_email.unwrap(),
			// 	verified_domain.name,
			// );
		}
		// TODO delete certificates and managed urls after 3 days
	}

	Ok(())
}

async fn verify_transferred_domain_to_patr() -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Verifying unverified domains", request_id);
	let config = super::CONFIG.get().unwrap();
	let mut connection = config.database.acquire().await?;

	let settings = config.config.clone();

	let unverified_transferred_domains =
		db::get_all_unverified_transferred_domains(&mut connection).await?;

	let client = service::get_cloudflare_client(&config.config).await?;

	for unverified_domain in unverified_transferred_domains {
		let mut connection = connection.begin().await?;

		let workspace_id =
			db::get_resource_by_id(&mut connection, &unverified_domain.id)
				.await?
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?
				.owner_id;
		let response = client
			.request(&zone::ZoneDetails {
				identifier: &unverified_domain.zone_identifier,
			})
			.await?;

		if let Status::Active = response.result.status {
			// Create certs below
		} else {
			continue;
		}

		// Delete user_transferring_domain_to_patr
		db::delete_user_transferred_domain_by_id(
			&mut connection,
			&unverified_domain.id,
		)
		.await?;

		// Delete user_controlled_domain
		db::delete_user_contolled_domain(
			&mut connection,
			&unverified_domain.id,
		)
		.await?;

		// Update ns type for workspace_domain
		db::update_workspace_domain_nameserver_type(
			&mut connection,
			&unverified_domain.id,
		)
		.await?;

		// Domain verified
		db::update_workspace_domain_status(
			&mut connection,
			&unverified_domain.id,
			true,
		)
		.await?;

		// Domain is now verified add to patr controlled domain
		db::add_patr_controlled_domain(
			&mut connection,
			&unverified_domain.id,
			&unverified_domain.zone_identifier,
		)
		.await?;

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

		let managed_urls_count = db::get_active_managed_url_count_for_domain(
			&mut connection,
			&unverified_domain.id,
		)
		.await?;

		if managed_urls_count > 0 {
			// Wait for k8s verification is done for create certificate
			// If done delete certificate for existing managed urls and
			// Use new certificate for domain for managed urls
			let is_certificate_ready =
				service::get_kubernetes_certificate_status(
					&unverified_domain.id,
					workspace_id.as_str(),
					&settings,
				)
				.await?;
			if is_certificate_ready {
				let managed_urls = db::get_all_managed_urls_for_domain(
					&mut connection,
					&unverified_domain.id,
				)
				.await?
				.into_iter()
				.filter_map(|url| {
					Some(ManagedUrl {
						id: url.id,
						sub_domain: url.sub_domain,
						domain_id: url.domain_id,
						path: url.path,
						url_type: match url.url_type {
							DbManagedUrlType::ProxyToDeployment => {
								ManagedUrlType::ProxyDeployment {
									deployment_id: url.deployment_id?,
									port: url.port? as u16,
								}
							}
							DbManagedUrlType::ProxyToStaticSite => {
								ManagedUrlType::ProxyStaticSite {
									static_site_id: url.static_site_id?,
								}
							}
							DbManagedUrlType::ProxyUrl => {
								ManagedUrlType::ProxyUrl { url: url.url? }
							}
							DbManagedUrlType::Redirect => {
								ManagedUrlType::Redirect { url: url.url? }
							}
						},
					})
				})
				.collect::<Vec<ManagedUrl>>();

				// Delete certificate for existing managed urls
				for managed_url in &managed_urls {
					service::delete_certificate(
						&workspace_id,
						&format!("certificate-{}", managed_url.id),
						&format!("tls-{}", managed_url.id),
						&settings,
						&request_id,
					)
					.await?;
				}
				// Update managed_urls with new certificate
				for managed_url in managed_urls {
					service::update_kubernetes_managed_url(
						&workspace_id,
						&managed_url,
						&settings,
						&request_id,
					)
					.await?;
				}
			}
		} else {
			connection.commit().await?;
			continue;
		}

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

		connection.commit().await?;
	}

	Ok(())
}
