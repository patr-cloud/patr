use crate::{
	models::rabbitmq::DockerRegistryData,
	service,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn process_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	request_data: DockerRegistryData,
	config: &Settings,
) -> Result<(), Error> {
	match request_data {
		DockerRegistryData::DeleteDockerImage {
			workspace_id,
			repository_name,
			digest,
			tag,
			image_pushed_ip_addr,
			request_id,
		} => {
			log::trace!("request_id: {request_id} - Deleting docker image from `{repository_name}` with digest `{digest}`");
			service::delete_docker_repository_image_in_registry(
				connection,
				&repository_name,
				&digest,
				config,
				&request_id,
			)
			.await?;

			log::trace!("request_id: {request_id} - Sending docker repo storage limit exceeded notification email to user");
			service::send_repository_storage_limit_exceed_email(
				connection,
				&workspace_id,
				&repository_name,
				&tag,
				&digest,
				&image_pushed_ip_addr,
			)
			.await?;
			Ok(())
		}
	}
}
