use tokio::process::Command;

use crate::{
	db,
	models::rabbitmq::BYOCData,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn process_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	request_data: BYOCData,
	config: &Settings,
) -> Result<(), Error> {
	match request_data {
		BYOCData::SetupKubernetesCluster {
			region_id,
			certificate_authority_data,
			auth_username,
			auth_token,
			request_id,
		} => {
			let region = if let Some(region) =
				db::get_region_by_id(connection, &region_id).await?
			{
				region
			} else {
				log::error!(
					"request_id: {} - Unable to find region with ID `{}`",
					request_id,
					&region_id
				);
				return Ok(());
			};

			install_with_helm().await?;
			Ok(())
		}
		BYOCData::CreateDigitaloceanCluster {
			region_id: _,
			digitalocean_region: _,
			access_token: _,
			request_id: _,
		} => Ok(()),
	}
}

async fn install_with_helm(
	local_name: &str,
	repo_name: &str,
) -> Result<(), Error> {
	Command::new("helm")
		.arg("install")
		.arg(local_name)
		.arg(repo_name)
		.arg("--kube-apiserver");
	Ok(())
}
