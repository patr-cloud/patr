use api_models::utils::Uuid;
use cloudflare::{
	endpoints::{
		workers::{self, CreateRouteParams},
		workerskv,
		zone::{
			certificates::{
				CertificateRequestType,
				CertificateRequestedValidity,
				CreateCertifcate,
				CreateCertifcateBody,
				RevokeCertificate,
			},
			custom_hostname::{
				ActivationStatus,
				CreateCustomHostname,
				CreateCustomHostnameParams,
				DeleteCustomHostname,
				EditCustomHostname,
				EditCustomHostnameParams,
				SslParams,
				SslSettingsParams,
			},
		},
	},
	framework::{
		async_api::Client as CloudflareClient,
		auth::Credentials,
		response::ApiFailure,
		Environment,
		HttpApiClientConfig,
	},
};
use eve_rs::AsError;

use crate::{
	db,
	models::cloudflare::{
		deployment,
		routing::{self, RouteType, UrlType},
		static_site,
	},
	utils::{settings::Settings, Error},
	Database,
};

const DELETION_KV_TTL_IN_SECS: i64 = 15 * 24 * 60 * 60; // 15 days

pub async fn get_cloudflare_client(
	config: &Settings,
) -> Result<CloudflareClient, Error> {
	let credentials = Credentials::UserAuthToken {
		token: config.cloudflare.api_token.clone(),
	};

	CloudflareClient::new(
		credentials,
		HttpApiClientConfig::default(),
		Environment::Production,
	)
	.map_err(|err| {
		log::error!("Error while initializing cloudflare client: {}", err);
		Error::empty()
	})
}

async fn update_kv_for_routing(
	key: routing::Key,
	value: routing::Value,
	config: &Settings,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config).await?;
	cf_client
		.request(&workerskv::write_bulk::WriteBulk {
			account_identifier: &config.cloudflare.account_id,
			namespace_identifier: &config.cloudflare.kv_routing_ns,
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

async fn delete_kv_for_routing(
	key: routing::Key,
	config: &Settings,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config).await?;
	cf_client
		.request(&workerskv::delete_key::DeleteKey {
			account_identifier: &config.cloudflare.account_id,
			namespace_identifier: &config.cloudflare.kv_routing_ns,
			key: &key.to_string(),
		})
		.await?;

	Ok(())
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

	let all_managed_urls_for_host_in_sorted_order =
		db::get_all_managed_urls_for_host(connection, sub_domain, domain_id)
			.await?
			.into_iter()
			.filter_map(|managed_url| {
				let path = managed_url.path;
				let url_type = match managed_url.url_type {
					db::ManagedUrlType::ProxyToDeployment => {
						UrlType::ProxyDeployment {
							deployment_id: managed_url.deployment_id?,
							port: managed_url.port.and_then(|port| {
								TryFrom::try_from(port).ok()
							})?,
						}
					}
					db::ManagedUrlType::ProxyToStaticSite => {
						UrlType::ProxyStaticSite {
							static_site_id: managed_url.static_site_id?,
						}
					}
					db::ManagedUrlType::ProxyUrl => UrlType::ProxyUrl {
						url: managed_url.url?,
						http_only: managed_url.http_only?,
					},
					db::ManagedUrlType::Redirect => UrlType::Redirect {
						url: managed_url.url?,
						http_only: managed_url.http_only?,
						permanent_redirect: managed_url.permanent_redirect?,
					},
				};

				Some(RouteType { path, url_type })
			})
			.collect::<Vec<_>>();

	let key = routing::Key {
		sub_domain: sub_domain.to_owned(),
		domain: domain.name,
	};

	// if no managed url is present, then delete the already existing key
	if all_managed_urls_for_host_in_sorted_order.is_empty() {
		delete_kv_for_routing(key, config).await?;
	} else {
		let value = routing::Value(all_managed_urls_for_host_in_sorted_order);
		update_kv_for_routing(key, value, config).await?;
	}

	Ok(())
}

pub async fn update_cloudflare_kv_for_deployment(
	deployment_id: &Uuid,
	value: deployment::Value,
	config: &Settings,
) -> Result<(), Error> {
	let key = deployment::Key(deployment_id.to_owned());

	let expiration_ttl = if value == deployment::Value::Deleted {
		Some(DELETION_KV_TTL_IN_SECS)
	} else {
		None
	};

	let cf_client = get_cloudflare_client(config).await?;
	cf_client
		.request(&workerskv::write_bulk::WriteBulk {
			account_identifier: &config.cloudflare.account_id,
			namespace_identifier: &config.cloudflare.kv_deployment_ns,
			bulk_key_value_pairs: vec![workerskv::write_bulk::KeyValuePair {
				key: key.to_string(),
				value: serde_json::to_string(&value)?,
				expiration_ttl,
				expiration: None,
				base64: None,
			}],
		})
		.await?;

	Ok(())
}

pub async fn update_cloudflare_kv_for_static_site(
	static_site_id: &Uuid,
	value: static_site::Value,
	config: &Settings,
) -> Result<(), Error> {
	let key = static_site::Key(static_site_id.to_owned());

	let expiration_ttl = if value == static_site::Value::Deleted {
		Some(DELETION_KV_TTL_IN_SECS)
	} else {
		None
	};

	let cf_client = get_cloudflare_client(config).await?;
	cf_client
		.request(&workerskv::write_bulk::WriteBulk {
			account_identifier: &config.cloudflare.account_id,
			namespace_identifier: &config.cloudflare.kv_static_site_ns,
			bulk_key_value_pairs: vec![workerskv::write_bulk::KeyValuePair {
				key: key.to_string(),
				value: serde_json::to_string(&value)?,
				expiration_ttl,
				expiration: None,
				base64: None,
			}],
		})
		.await?;

	Ok(())
}

pub async fn add_domain_to_cloudflare_worker_routes(
	host: &str,
	config: &Settings,
) -> Result<String, Error> {
	let cf_client = get_cloudflare_client(config).await?;

	let response = cf_client
		.request(&workers::CreateRoute {
			zone_identifier: &config.cloudflare.patr_zone_identifier,
			params: CreateRouteParams {
				pattern: format!("*{}/*", host),
				script: Some(config.cloudflare.worker_script.to_owned()),
			},
		})
		.await?;

	Ok(response.result.id)
}

pub async fn delete_domain_from_cloudflare_worker_routes(
	route_id: &str,
	config: &Settings,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config).await?;

	let response = cf_client
		.request(&workers::DeleteRoute {
			zone_identifier: &config.cloudflare.patr_zone_identifier,
			identifier: route_id,
		})
		.await;
	match response {
		// Do nothing
		Ok(_) => (),
		// Do nothing
		Err(ApiFailure::Error(code, errors))
			if code == 404 &&
				errors.errors.iter().any(|error| error.code == 10009) => {}
		Err(error) => return Err(error.into()),
	}

	Ok(())
}

pub async fn add_custom_hostname_to_cloudflare(
	host: &str,
	config: &Settings,
) -> Result<(String, ActivationStatus), Error> {
	let cf_client = get_cloudflare_client(config).await?;

	let response = cf_client
		.request(&CreateCustomHostname {
			zone_identifier: &config.cloudflare.patr_zone_identifier,
			params: CreateCustomHostnameParams {
				hostname: host.to_owned(),
				ssl: SslParams {
					method: "http".to_owned(),
					type_: "dv".to_owned(),
					settings: SslSettingsParams {
						min_tls_version: "1.0".to_owned(),
						..Default::default()
					},
					..Default::default()
				},
			},
		})
		.await?;

	Ok((response.result.id, response.result.status))
}

pub async fn delete_custom_hostname_from_cloudflare(
	custom_hostname_id: &str,
	config: &Settings,
) -> Result<(), Error> {
	let cf_client = get_cloudflare_client(config).await?;

	cf_client
		.request(&DeleteCustomHostname {
			zone_identifier: &config.cloudflare.patr_zone_identifier,
			identifier: custom_hostname_id,
		})
		.await?;

	Ok(())
}

pub async fn refresh_custom_hostname_in_cloudflare(
	custom_hostname_id: &str,
	config: &Settings,
) -> Result<ActivationStatus, Error> {
	let cf_client = get_cloudflare_client(config).await?;

	let response = cf_client
		.request(&EditCustomHostname {
			zone_identifier: &config.cloudflare.patr_zone_identifier,
			identifier: custom_hostname_id,
			params: EditCustomHostnameParams {
				ssl: SslParams {
					method: "http".to_owned(),
					type_: "dv".to_owned(),
					settings: SslSettingsParams {
						min_tls_version: "1.0".to_owned(),
						..Default::default()
					},
					..Default::default()
				},
			},
		})
		.await?;

	Ok(response.result.status)
}

pub struct CfCertificate {
	pub id: String,
	pub cert: String,
	pub key: String,
}

pub async fn create_origin_ca_certificate_for_region(
	region_id: &Uuid,
	config: &Settings,
) -> Result<CfCertificate, Error> {
	let hostnames = vec![
		format!("{}.{}", region_id, config.cloudflare.onpatr_domain),
		format!("*.{}.{}", region_id, config.cloudflare.onpatr_domain),
	];

	let cert = rcgen::generate_simple_self_signed(hostnames.clone())?;

	// for origin ca, origin_ca_key should be used for client
	let cf_client = {
		let credentials = Credentials::Service {
			key: config.cloudflare.origin_ca_key.clone(),
		};

		CloudflareClient::new(
			credentials,
			HttpApiClientConfig::default(),
			Environment::Production,
		)
		.map_err(|err| {
			log::error!("Error while initializing cloudflare client: {}", err);
			Error::empty()
		})?
	};

	let response = cf_client
		.request(&CreateCertifcate {
			body: CreateCertifcateBody {
				csr: cert.serialize_request_pem()?,
				hostnames,
				request_type: CertificateRequestType::OriginEcc,
				requested_validity: CertificateRequestedValidity::Days_5475,
			},
		})
		.await?;

	Ok(CfCertificate {
		id: response.result.id,
		cert: response.result.certificate,
		key: cert.serialize_private_key_pem(),
	})
}

pub async fn revoke_origin_ca_certificate(
	certificate_id: &str,
	config: &Settings,
) -> Result<(), Error> {
	// for origin ca, origin_ca_key should be used for client
	let cf_client = {
		let credentials = Credentials::Service {
			key: config.cloudflare.origin_ca_key.clone(),
		};

		CloudflareClient::new(
			credentials,
			HttpApiClientConfig::default(),
			Environment::Production,
		)
		.map_err(|err| {
			log::error!("Error while initializing cloudflare client: {}", err);
			Error::empty()
		})?
	};

	cf_client
		.request(&RevokeCertificate {
			identifier: certificate_id,
		})
		.await?;

	Ok(())
}
