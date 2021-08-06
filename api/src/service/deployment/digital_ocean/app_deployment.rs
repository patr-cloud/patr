use std::ops::DerefMut;

use eve_rs::AsError;
use reqwest::Client;

use crate::{
	db,
	error,
	models::deployment::cloud_providers::digital_ocean::{
		AppConfig,
		AppHolder,
		AppSpec,
		Domains,
		Image,
		Routes,
		Services,
	},
	service,
	utils::{settings::Settings, Error},
};

// create a new digital ocean application
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
						// for now this has been set to PRIMARY
						r#type: "PRIMARY".to_string(),
					}],
					services: vec![Services {
						name: "default-service".to_string(),
						image: Image {
							registry_type: "DOCR".to_string(),
							repository: hex::encode(deployment_id),
							tag: tag.to_string(),
						},
						// for now instance count is set to 1
						instance_count: 1,
						instance_size_slug: "basic-xs".to_string(),
						http_port: 80,
						routes: vec![Routes {
							path: "/".to_string(),
						}],
					}],
				}
			},
		})
		.send()
		.await?
		.json::<AppHolder>()
		.await?;

	if deploy_app.app.id.is_empty() {
		Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}

	let app = service::get_app().clone();

	db::update_deployment_table_with_live_deployment_id(
		app.database.acquire().await?.deref_mut(),
		&deploy_app.app.id,
		deployment_id,
	)
	.await?;

	Ok(())
}

pub async fn create_digital_ocean_deployment(
	config: &Settings,
	deployment_id: &[u8],
) -> Result<(), Error> {
	let app = service::get_app().clone();

	let deployment = db::get_deployment_by_id(
		app.database.acquire().await?.deref_mut(),
		deployment_id,
	)
	.await?;

	if let Some(deployment_details) = deployment {
		if let Some(app_deployment_id) = deployment_details.deployment_id {
			// for now i am keeping this as status and not mapping the the value
			// to struct as i couldn't find any use for it
			let deployment_info = Client::new()
				.get(format!(
					"https://api.digitalocean.com/v2/apps/{}/deployments",
					app_deployment_id
				))
				.bearer_auth(&config.digital_ocean_api_key)
				.send()
				.await?
				.status();

			if deployment_info.is_client_error() ||
				deployment_info.is_server_error()
			{
				Error::as_result()
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?;
			}
		} else {
			// maybe add new error here, deployment_id doesn't exist
			Error::as_result()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())?;
		}
	} else {
		// maybe add new error here, deployment doesn't exist
		Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
	}

	Ok(())
}
