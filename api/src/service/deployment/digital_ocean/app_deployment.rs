use eve_rs::AsError;
use reqwest::Client;

use crate::{
	error,
	models::deployment::cloud_providers::digital_ocean::{
		App,
		AppConfig,
		AppSpec,
		Domains,
		Image,
		Services,
	},
	utils::{settings::Settings, Error},
};

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
						r#type: "DEFAULT".to_string(),
						wildcard: false,
						zone: "vicara.tech".to_string(),
					}],
					services: Some(vec![Services {
						name: "default-service".to_string(),
						git: None,
						github: None,
						gitlab: None,
						image: Image {
							registry: None,
							registry_type: Some("DOCR".to_string()),
							repository: Some(hex::encode(deployment_id)),
							tag: Some(tag.to_string()),
						},
						dockerfile_path: None,
						build_command: None,
						run_command: None,
						source_dir: None,
						envs: None,
						environment_slug: None,
						instance_count: 1,
						instance_size_slug: "basic-xs".to_string(),
						cors: None,
						health_check: None,
						http_port: vec![80],
						internal_ports: None,
						routes: None,
					}]),
					static_sites: None,
					jobs: None,
					workers: None,
					databases: None,
				}
			},
		})
		.send()
		.await?
		.json::<App>()
		.await?;

	if deploy_app.id.is_empty() {
		Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}
	Ok(())
}
