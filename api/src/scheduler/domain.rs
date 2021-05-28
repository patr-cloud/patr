use cloudflare::{
	endpoints::zone::{self, Status, Zone},
	framework::{
		async_api::{ApiClient, Client},
		auth::Credentials,
		Environment,
		HttpApiClientConfig,
	},
};

use crate::{
	db,
	scheduler::Job,
	utils::{mailer, validator},
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

	let unverified_domains =
		db::get_all_unverified_domains(&mut connection).await?;

	let credentials = Credentials::UserAuthToken {
		token: config.config.cloudflare.api_token.clone(),
	};

	let client = Client::new(
		credentials,
		HttpApiClientConfig::default(),
		Environment::Production,
	)?;

	for unverified_domain in unverified_domains {
		let zone = get_zone_for_domain(&client, &unverified_domain.name).await;

		if zone.is_none() {
			// add to cflr
			log::error!(
				"Domain `{}` was not added to cloudflare. Adding again.",
				unverified_domain.name
			);
			let response = client
				.request(&zone::CreateZone {
					params: zone::CreateZoneParams {
						account: &config.config.cloudflare.account_id,
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
				mailer::send_domain_verified_mail(
					config.config.clone(),
					notification_email.unwrap(),
					unverified_domain.name,
				);
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

	let credentials = Credentials::UserAuthToken {
		token: config.config.cloudflare.api_token.clone(),
	};

	let client = Client::new(
		credentials,
		HttpApiClientConfig::default(),
		Environment::Production,
	)
	.unwrap();

	for verified_domain in verified_domains {
		let zone = get_zone_for_domain(&client, &verified_domain.name).await;

		if zone.is_none() {
			// add to cflr
			log::error!(
				"Domain `{}` was not added to cloudflare. Adding again.",
				verified_domain.name
			);
			let response = client
				.request(&zone::CreateZone {
					params: zone::CreateZoneParams {
						account: &config.config.cloudflare.account_id,
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
		let zone = zone.unwrap();

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
		mailer::send_domain_unverified_mail(
			config.config.clone(),
			notification_email.unwrap(),
			verified_domain.name,
		);
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
