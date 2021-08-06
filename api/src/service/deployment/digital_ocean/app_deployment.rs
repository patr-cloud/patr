use eve_rs::AsError;
use reqwest::Client;

use crate::{error, models::deployment::cloud_providers::digital_ocean::{App, AppConfig, AppSpec, Domains, Image, Routes, Services}, utils::{settings::Settings, Error}};

pub async fn create_digital_ocean_application(
	settings: &Settings,
	deployment_id: &[u8],
	tag: &str,
) -> Result<(), Error> {
	let deploy_app = Client::new()
		.post("https://api.digitalocean.com/v2/apps")
		.bearer_auth(&settings.digital_ocean_api_key)
		.json(&AppConfig {
			spec: {
				AppSpec {
					name: hex::encode(&deployment_id),
					region: "blr".to_string(),
					domains: vec![Domains {
						// [ 4 .. 253 ] characters
						// ^((xn--)?[a-zA-Z0-9]+(-[a-zA-Z0-9]+)*\.)+[a-zA-Z]{2,
						// }\.?$ The hostname for the domain
						domain: format!(
							"{}.vicara.tech",
							hex::encode(deployment_id)
						),
						r#type: "PRIMARY".to_string(),
						wildcard: false
					}],
					services: vec![Services {
						name: "default-service".to_string(),
						image: Image {
							registry: "".to_string(),
							registry_type: "DOCR".to_string(),
							repository: hex::encode(deployment_id),
							tag: tag.to_string(),
						},
						instance_count: 1,
						instance_size_slug: "basic-xs".to_string(),
						http_port: 80,
						routes: vec![Routes{
							path: "/".to_string()
						}]
					}]
				}
			}
		})
		.send()
		.await?;

	let deploy_app = deploy_app.text().await?;

	println!("{:?}", deploy_app);

	// if deploy_app.id.is_empty() {
	// 	Error::as_result()
	// 		.status(500)
	// 		.body(error!(SERVER_ERROR).to_string())?;
	// }
	Ok(())
}
