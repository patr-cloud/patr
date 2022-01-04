use api_models::{
	models::workspace::infrastructure::deployment::DeploymentStatus,
	utils::Uuid,
};
use eve_rs::AsError;

use crate::{db, error, service, utils::Error, Database};

pub struct DockerRepository {
	pub id: Uuid,
	pub workspace_id: Uuid,
	pub name: String,
}

pub struct Deployment {
	pub id: Uuid,
	pub name: String,
	pub registry: String,
	pub repository_id: Option<Uuid>,
	pub image_name: Option<String>,
	pub image_tag: String,
	pub status: DeploymentStatus,
	pub workspace_id: Uuid,
	pub region: Uuid,
	pub min_horizontal_scale: i16,
	pub max_horizontal_scale: i16,
	pub machine_type: Uuid,
	pub deploy_on_push: bool,
}

impl Deployment {
	pub async fn get_full_image(
		&self,
		connection: &mut <Database as sqlx::Database>::Connection,
	) -> Result<String, Error> {
		if self.registry == "registry.patr.cloud" {
			let docker_repository = db::get_docker_repository_by_id(
				&mut *connection,
				self.repository_id
					.as_ref()
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?,
			)
			.await?
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

			let workspace = db::get_workspace_info(
				&mut *connection,
				&docker_repository.workspace_id,
			)
			.await?
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

			Ok(format!(
				"{}/{}/{}",
				service::get_settings().docker_registry.registry_url,
				workspace.name,
				docker_repository.name
			))
		} else {
			Ok(format!(
				"{}/{}",
				self.registry,
				self.image_name
					.as_ref()
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?
			))
		}
	}
}
