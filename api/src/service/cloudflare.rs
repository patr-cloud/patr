use std::collections::HashMap;

use api_models::utils::Uuid;
use cloudflare::{
	endpoints::workerskv,
	framework::{
		async_api::Client as CloudflareClient,
		auth::Credentials,
		Environment,
		HttpApiClientConfig,
	},
};
use eve_rs::AsError;

use crate::{
	db,
	models::cloudflare::{
		deployment,
		routing::{self, ManagedUrlType},
		static_site,
	},
	utils::{settings::Settings, Error},
	Database,
};

pub async fn get_cloudflare_client(
	config: &Settings,
) -> Result<CloudflareClient, Error> {
	let credentials = Credentials::UserAuthToken {
		token: config.cloudflare.api_token.clone(),
	};

	let client = if let Ok(client) = CloudflareClient::new(
		credentials,
		HttpApiClientConfig::default(),
		Environment::Production,
	) {
		client
	} else {
		return Err(Error::empty());
	};
	Ok(client)
}

pub async fn update_cloudflare_kv_for_managed_url(
	connection: &mut <Database as sqlx::Database>::Connection,
	sub_domain: &str,
	domain_id: &Uuid,
	config: &Settings,
) -> Result<(), Error> {
	let domain = db::get_workspace_domain_by_id(connection, domain_id)
		.await?
		.status(500)?;

	let all_managed_urls_for_host =
		db::get_all_managed_urls_for_host(connection, sub_domain, domain_id)
			.await?
			.into_iter()
			.filter_map(|managed_url| {
				let key = managed_url.path;
				let value = match managed_url.url_type {
					db::ManagedUrlType::ProxyToDeployment => {
						ManagedUrlType::ProxyDeployment {
							deployment_id: managed_url.deployment_id?,
						}
					}
					db::ManagedUrlType::ProxyToStaticSite => {
						ManagedUrlType::ProxyStaticSite {
							static_site_id: managed_url.static_site_id?,
						}
					}
					db::ManagedUrlType::ProxyUrl => ManagedUrlType::ProxyUrl {
						url: managed_url.url?,
					},
					db::ManagedUrlType::Redirect => ManagedUrlType::Redirect {
						url: managed_url.url?,
					},
				};

				Some((key, value))
			})
			.collect::<HashMap<_, _>>();

	let cf_client = get_cloudflare_client(config).await?;
	let key = routing::Key {
		sub_domain: sub_domain.to_owned(),
		domain: domain.name,
	};

	// if no managed url is present, then delete the already existing key
	if all_managed_urls_for_host.is_empty() {
		cf_client
			.request_handle(&workerskv::delete_key::DeleteKey {
				account_identifier: &config.cloudflare.account_id,
				namespace_identifier: &config.cloudflare.kv_routing_ns,
				key: &key.to_string(),
			})
			.await?;
	} else {
		let value = routing::Value(all_managed_urls_for_host);

		cf_client
			.request_handle(&workerskv::write_bulk::WriteBulk {
				account_identifier: &config.cloudflare.account_id,
				namespace_identifier: &config.cloudflare.kv_routing_ns,
				bulk_key_value_pairs: vec![
					workerskv::write_bulk::KeyValuePair {
						key: key.to_string(),
						value: serde_json::to_string(&value)?,
						expiration: None,
						expiration_ttl: None,
						base64: None,
					},
				],
			})
			.await?;
	}

	Ok(())
}

pub async fn update_cloudflare_kv_for_deployment(
	deployment_id: &Uuid,
	region_id: &Uuid,
	exposted_ports: &[u16],
	config: &Settings,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config).await?;
	let key = deployment::Key(deployment_id.to_owned());

	let value = deployment::Value {
		region_id: region_id.to_owned(),
		ports: exposted_ports.to_vec(),
	};

	cf_client
		.request_handle(&workerskv::write_bulk::WriteBulk {
			account_identifier: &config.cloudflare.account_id,
			namespace_identifier: &config.cloudflare.kv_deployment_ns,
			bulk_key_value_pairs: vec![workerskv::write_bulk::KeyValuePair {
				key: key.to_string(),
				value: serde_json::to_string(&value)?,
				expiration: None,
				expiration_ttl: None,
				base64: None,
			}],
		})
		.await?;

	Ok(())
}

pub async fn delete_cloudflare_kv_for_deployment(
	deployment_id: &Uuid,
	config: &Settings,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config).await?;
	let key = deployment::Key(deployment_id.to_owned());

	cf_client
		.request_handle(&workerskv::delete_key::DeleteKey {
			account_identifier: &config.cloudflare.account_id,
			namespace_identifier: &config.cloudflare.kv_deployment_ns,
			key: &key.to_string(),
		})
		.await?;

	Ok(())
}

pub async fn update_cloudflare_kv_for_static_site(
	static_site_id: &Uuid,
	upload_id: &Uuid,
	config: &Settings,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config).await?;
	let key = static_site::Key(static_site_id.to_owned());
	let value = static_site::Value {
		upload_id: upload_id.to_owned(),
	};

	cf_client
		.request_handle(&workerskv::write_bulk::WriteBulk {
			account_identifier: &config.cloudflare.account_id,
			namespace_identifier: &config.cloudflare.kv_static_site_ns,
			bulk_key_value_pairs: vec![workerskv::write_bulk::KeyValuePair {
				key: key.to_string(),
				value: serde_json::to_string(&value)?,
				expiration: None,
				expiration_ttl: None,
				base64: None,
			}],
		})
		.await?;

	Ok(())
}

pub async fn delete_cloudflare_kv_for_static_site(
	static_site_id: &Uuid,
	config: &Settings,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config).await?;
	let key = static_site::Key(static_site_id.to_owned());

	cf_client
		.request_handle(&workerskv::delete_key::DeleteKey {
			account_identifier: &config.cloudflare.account_id,
			namespace_identifier: &config.cloudflare.kv_static_site_ns,
			key: &key.to_string(),
		})
		.await?;

	Ok(())
}
