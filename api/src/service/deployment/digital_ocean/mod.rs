mod app_deployment;

use std::process::{Command, Stdio};

pub use app_deployment::*;
use eve_rs::AsError;
use serde_json::json;

use crate::{
	models::error::{id as ErrorId, message as ErrorMessage},
	utils::{constants::request_keys, settings::Settings, Error},
};

pub async fn push_to_digital_ocean_registry(
	image_name: &str,
	tag: &str,
	deployment_id: &[u8],
	settings: Settings,
) -> Result<(), Error> {
	let output = Command::new("doctl")
		.arg("registry")
		.arg("login")
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.expect("failed to execute process")
		.wait()
		.expect("failed to login");

	if output.success() {
		let docker_image = Command::new("docker")
			.arg("tag")
			.arg(&image_name)
			.arg(format!(
				"registry.digitalocean.com/project-apex/{:?}",
				hex::encode(&deployment_id)
			))
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn()
			.expect("failed to execute process")
			.wait()
			.expect("failed to tag the image");

		if docker_image.success() {
			let push_image = Command::new("docker")
				.arg("push")
				.arg(format!(
					"registry.digitalocean.com/project-apex/{:?}",
					hex::encode(&deployment_id)
				))
				.stdout(Stdio::piped())
				.stderr(Stdio::piped())
				.spawn()
				.expect("failed to execute process")
				.wait()
				.expect("failed to push the image");
			
            if push_image.success() {
				println!("Image pushed successfully!");
				// if the digital ocean app doesn't exists then update the app
				if !digital_ocean_app_exists() {
					create_digital_ocean_application(
						&settings,
						&deployment_id,
						&tag,
					)
					.await?;
				}
			} else {
				Error::as_result().status(500).body(
					json!({
						request_keys::ERRORS: [{
							request_keys::CODE: ErrorId::SERVER_ERROR,
							request_keys::MESSAGE: ErrorMessage::LOGIN_FAILURE,
							request_keys::DETAIL: []
						}]
					})
					.to_string(),
				)?;
			}
		} else {
			Error::as_result().status(500).body(
				json!({
					request_keys::ERRORS: [{
						request_keys::CODE: ErrorId::SERVER_ERROR,
						request_keys::MESSAGE: ErrorMessage::LOGIN_FAILURE,
						request_keys::DETAIL: []
					}]
				})
				.to_string(),
			)?;
		}
	} else {
		Error::as_result().status(500).body(
			json!({
				request_keys::ERRORS: [{
					request_keys::CODE: ErrorId::SERVER_ERROR,
					request_keys::MESSAGE: ErrorMessage::LOGIN_FAILURE,
					request_keys::DETAIL: []
				}]
			})
			.to_string(),
		)?;
	}
	Ok(())
}

pub fn digital_ocean_app_exists() -> bool {
	return false;
}
