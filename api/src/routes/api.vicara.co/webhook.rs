use std::{collections::HashMap, time::Duration};

use cloudflare::{
	endpoints::{
		dns::{
			CreateDnsRecord,
			CreateDnsRecordParams,
			DnsContent,
			ListDnsRecords,
			ListDnsRecordsParams,
			UpdateDnsRecord,
			UpdateDnsRecordParams,
		},
		zone::{ListZones, ListZonesParams},
	},
	framework::{
		async_api::{ApiClient, Client},
		auth::Credentials,
		Environment,
		HttpApiClientConfig,
	},
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use futures::StreamExt;
use hex::ToHex;
use rand::Rng;
use shiplift::{ContainerOptions, Docker, PullOptions, RegistryAuth};
use tokio::{fs, process::Command};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{db_mapping::EventData, rbac, RegistryToken, RegistryTokenAccess},
	pin_fn,
	utils::{get_current_time, Error, ErrorData, EveContext, EveMiddleware},
};

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut sub_app = create_eve_app(app);

	sub_app.post(
		"/docker-registry/notification",
		[EveMiddleware::CustomFunction(pin_fn!(notification_handler))],
	);
	sub_app
}

pub async fn notification_handler(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	if context.get_content_type().as_str() !=
		"application/vnd.docker.distribution.events.v1+json"
	{
		Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
	}
	let body = context.get_body()?;
	let events: EventData = serde_json::from_str(&body)?;
	let config = context.get_state().clone().config;

	let server_ip = "128.199.25.235";

	// init docker
	let docker = Docker::new();

	// check if the event is a push event
	// get image name, repository name, tag if present
	for event in events.events {
		if event.action != "push" {
			continue;
		}
		let target = event.target;
		if target.tag.is_empty() {
			continue;
		}

		let repository = target.repository;
		let mut splitter = repository.split('/');
		let org_name = if let Some(val) = splitter.next() {
			val
		} else {
			continue;
		};
		let image_name = if let Some(val) = splitter.next() {
			val
		} else {
			continue;
		};
		let tag = target.tag;

		let organisation = db::get_organisation_by_name(
			context.get_mysql_connection(),
			org_name,
		)
		.await?;
		if organisation.is_none() {
			continue;
		}
		let organisation = organisation.unwrap();

		let deployments =
			db::get_deployments_by_image_name_and_tag_for_organisation(
				context.get_mysql_connection(),
				image_name,
				&tag,
				&organisation.id,
			)
			.await?;

		for deployment in deployments {
			let container_name =
				format!("deployment-{}", deployment.id.encode_hex::<String>());
			let full_image_name = format!(
				"{}/{}/{}@{}",
				config.docker_registry.registry_url,
				org_name,
				deployment.image_name,
				target.digest
			);

			// Pull the latest image again
			let god_user = db::get_user_by_user_id(
				context.get_mysql_connection(),
				rbac::GOD_USER_ID.get().unwrap().as_bytes(),
			)
			.await?
			.unwrap();
			let god_username = god_user.username;
			// generate token as password
			let iat = get_current_time().as_secs();
			let generated_password = RegistryToken::new(
				config.docker_registry.issuer.clone(),
				iat,
				god_username.clone(),
				&config,
				vec![RegistryTokenAccess {
					r#type: "repository".to_string(),
					name: repository.to_string(),
					actions: vec!["pull".to_string()],
				}],
			)
			.to_string(
				config.docker_registry.private_key.as_ref(),
				config.docker_registry.public_key_der(),
			);
			if let Err(err) = generated_password {
				log::error!("Error generating docker CLI token: {}", err);
				continue;
			}
			let token = generated_password.unwrap();

			// get token object using the above token string
			let registry_auth = RegistryAuth::builder()
				.username(god_username)
				.password(token)
				.build();
			let mut stream = docker.images().pull(
				&PullOptions::builder()
					.image(&full_image_name)
					.auth(registry_auth)
					.build(),
			);
			while stream.next().await.is_some() {}

			let empty_vec = vec![];
			let empty_map = HashMap::new();
			let empty_string = String::default();

			// If the container exists, stop it and delete it
			// The errors will be taken care of by the `unwrap_or_default` part
			let container = docker.containers().get(&container_name);
			let info = container.inspect().await;

			let mut port;

			if let Ok(info) = info {
				// don't redeploy the image if it's already deployed
				if info.config.image == full_image_name {
					log::debug!(
						"Pushed image is already deployed. Ignoring..."
					);
					continue;
				}
				docker
					.containers()
					.get(&container_name)
					.stop(Some(Duration::from_secs(5)))
					.await
					.unwrap_or_default();
				docker
					.containers()
					.get(&container_name)
					.delete()
					.await
					.unwrap_or_default();
				port = info
					.host_config
					.port_bindings
					.unwrap_or_default()
					.get(&format!("{}/tcp", deployment.port))
					.unwrap_or(&empty_vec)
					.get(0)
					.unwrap_or_else(|| &empty_map)
					.get("HostPort")
					.unwrap_or_else(|| &empty_string)
					.parse()
					.unwrap_or(0);
			} else {
				port = 0;
			}

			if port == 0 {
				// Assign a random, available port
				let low = 1025;
				let high = 65535;
				let restricted_ports = [5800, 8080, 9000, 5000, 3000];
				loop {
					port = rand::thread_rng().gen_range(low..high);
					if restricted_ports.contains(&port) {
						continue;
					}
					let port_open = port_scanner::scan_port_addr(format!(
						"{}:{}",
						"0.0.0.0", port
					));
					if port_open {
						continue;
					}
					break;
				}
			}

			let container = docker
				.containers()
				.create(
					&ContainerOptions::builder(&full_image_name)
						.name(&container_name)
						.privileged(false)
						.expose(deployment.port as u32, "tcp", port)
						.build(),
				)
				.await;
			if let Err(err) = container {
				log::error!("Error creating container: {:?}", err);
				// TODO log somewhere that the creation failed and that it
				// needs to be done again
				continue;
			}

			let result = docker.containers().get(&container_name).start().await;
			if let Err(err) = result {
				log::error!("Error starting container: {:?}", err);
				// TODO log somewhere that the start failed and that it
				// needs to be done again
				continue;
			}

			// TODO clean up all this shit with service layer configs

			let credentials = Credentials::UserAuthToken {
				token: context.get_state().config.cloudflare.api_token.clone(),
			};

			let client = Client::new(
				credentials,
				HttpApiClientConfig::default(),
				Environment::Production,
			)
			.unwrap();

			let domain = format!("{}.vicara.tech", hex::encode(&deployment.id));
			let response = client
				.request(&ListZones {
					params: ListZonesParams {
						name: Some(String::from("vicara.tech")),
						..Default::default()
					},
				})
				.await;
			if let Err(err) = response {
				log::error!("Error listing zones: {:?}", err);
				continue;
			}
			let response = response.unwrap();
			let zone_id = response.result.get(0);
			if zone_id.is_none() {
				log::error!("Zero zones returned");
				continue;
			}
			let zone_identifier = zone_id.unwrap().id.as_str();
			let expected_dns_record = DnsContent::A {
				content: server_ip.parse().unwrap(),
			};

			let response = client
				.request(&ListDnsRecords {
					zone_identifier,
					params: ListDnsRecordsParams {
						name: Some(domain.clone()),
						..Default::default()
					},
				})
				.await;
			if let Err(err) = response {
				log::error!("Error listing DNS records: {:?}", err);
				continue;
			}
			let response = response.unwrap();
			let dns_record = response.result.iter().find(|record| {
				if let DnsContent::A { .. } = record.content {
					record.name == domain
				} else {
					false
				}
			});
			if let Some(record) = dns_record {
				let response = client
					.request(&UpdateDnsRecord {
						zone_identifier,
						identifier: record.id.as_str(),
						params: UpdateDnsRecordParams {
							content: expected_dns_record,
							name: domain.as_str(),
							proxied: Some(true),
							ttl: Some(1),
						},
					})
					.await;
				if let Err(err) = response {
					log::error!("Error updating DNS record: {:?}", err);
					continue;
				}
			} else {
				// Create
				let response = client
					.request(&CreateDnsRecord {
						zone_identifier,
						params: CreateDnsRecordParams {
							content: expected_dns_record,
							name: deployment.sub_domain.as_str(),
							ttl: Some(1),
							priority: None,
							proxied: Some(true),
						},
					})
					.await;
				if let Err(err) = response {
					log::error!("Error creating DNS record: {:?}", err);
					continue;
				}
			}

			let write_result = fs::write(
				format!(
					"/etc/nginx/sites-enabled/{}",
					hex::encode(&deployment.id)
				),
				format!(
					r#"
server {{
	listen 80;
	listen [::]:80;
	server_name {domain};
	
	return 301 https://{domain}$request_uri;
}}
server {{
	listen 443 ssl http2;
	listen [::]:443 ssl http2;
	server_name {domain};
	
	ssl_certificate /etc/letsencrypt/live/deployment.vicara.tech/fullchain.pem;
	ssl_certificate_key /etc/letsencrypt/live/deployment.vicara.tech/privkey.pem;
	
	location {path} {{
		proxy_pass http://localhost:{port};
	}}
	
	include snippets/letsencrypt.conf;
}}
"#,
					domain = domain,
					port = port,
					path = deployment.path,
				),
			)
			.await;
			if let Err(err) = write_result {
				log::error!("Error creating nginx conf : {:?}", err);
				// TODO log somewhere that the start failed and that it
				// needs to be done again
				continue;
			}
			let reload_result = Command::new("nginx")
				.arg("-s")
				.arg("reload")
				.spawn()
				.expect("unable to spawn nginx process")
				.wait()
				.await?;
			if !reload_result.success() {
				log::error!("Error reloading nginx : {:?}", reload_result);
				// TODO log somewhere that the start failed and that it
				// needs to be done again
				continue;
			}
		}
	}

	Ok(context)
}
