use crate::{db, utils::mailer};

use async_std::task;
use cloudflare::{
	endpoints::zone::{self, Status, Zone},
	framework::{
		apiclient::ApiClient,
		auth::Credentials,
		Environment,
		HttpApiClient,
		HttpApiClientConfig,
	},
};
use job_scheduler::Job;
use surf::mime::APPLICATION_JSON;

// Every two hours
pub fn verify_unverified_domains_job<'a>() -> Job<'a> {
	Job::new("0 0 1/2 * * *".parse().unwrap(), || {
		task::block_on(verify_unverified_domains()).unwrap_or_else(|err| {
			log::error!(
				"Error while trying to verify unverified domains: {}",
				err
			);
		});
	})
}

// Every day at 4 am
pub fn reverify_verified_domains_job<'a>() -> Job<'a> {
	Job::new("0 0 4 * * *".parse().unwrap(), || {
		task::block_on(reverify_verified_domains()).unwrap_or_else(|err| {
			log::error!(
				"Error while trying to verify unverified domains: {}",
				err
			);
		});
	})
}

async fn verify_unverified_domains() -> crate::Result<()> {
	let config = super::CONFIG.get().unwrap();
	let db_pool = &config.db_pool;
	let mut connection = db_pool.begin().await?;

	let unverified_domains =
		db::get_all_unverified_domains(&mut connection).await?;

	let credentials = Credentials::UserAuthToken {
		token: config.config.cloudflare.api_token.clone(),
	};

	let client = HttpApiClient::new(
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
			let response = client.request(&zone::CreateZone {
				params: zone::CreateZoneParams {
					account: &config.config.cloudflare.account_id,
					name: &unverified_domain.name,
					jump_start: Some(false),
					zone_type: Some(zone::Type::Full),
				},
			})?;
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

		let response = client.request(&zone::ZoneDetails {
			identifier: &zone.id,
		})?;

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
		let response = surf::put(format!(
			"https://api.cloudflare.com/client/v4/zones/{}/activation_check",
			response.result.id
		))
		.set_header(
			"Authorization",
			format!("Bearer {}", config.config.cloudflare.api_token),
		)
		.set_mime(APPLICATION_JSON)
		.await;
		if let Err(err) = response {
			log::error!("Cannot initiate zone activation check: {}", err);
		}
	}

	Ok(())
}

async fn reverify_verified_domains() -> crate::Result<()> {
	let config = super::CONFIG.get().unwrap();
	let db_pool = &config.db_pool;
	let mut connection = db_pool.begin().await?;

	let verified_domains =
		db::get_all_verified_domains(&mut connection).await?;

	let credentials = Credentials::UserAuthToken {
		token: config.config.cloudflare.api_token.clone(),
	};

	let client = HttpApiClient::new(
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
			let response = client.request(&zone::CreateZone {
				params: zone::CreateZoneParams {
					account: &config.config.cloudflare.account_id,
					name: &verified_domain.name,
					jump_start: Some(false),
					zone_type: Some(zone::Type::Full),
				},
			})?;
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

		let response = client.request(&zone::ZoneDetails {
			identifier: &zone.id,
		})?;

		if let Status::Active = response.result.status {
			continue;
		} else {
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
	}

	Ok(())
}

pub async fn get_zone_for_domain(
	client: &HttpApiClient,
	domain: &str,
) -> Option<Zone> {
	let response = if let Ok(response) = client.request(&zone::ListZones {
		params: zone::ListZonesParams {
			name: Some(domain.to_string()),
			..Default::default()
		},
	}) {
		response
	} else {
		return None;
	};

	response.result.into_iter().next()
}
