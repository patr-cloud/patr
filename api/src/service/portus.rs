// SERVICE FOR PORTUS
use shiplift::{Docker, Error};

// function to stop a container
pub async fn delete_container(
	docker: Docker,
	container_name: &str,
) -> Result<(), Error> {
	let container_stop_result =
		docker.containers().get(&container_name).stop(None).await;

	let container_delete_result =
		docker.containers().get(&container_name).delete().await;

	if container_delete_result.is_err() {
		let container_start_result =
			docker.containers().get(&container_name).start().await;
		if let Err(err) = container_start_result {
			log::error!(
				"Unrecoverable error while starting container: {:?}",
				err
			);

			return Err(err);
		}
	}
	if let Err(err) = container_stop_result.and(container_delete_result) {
		log::error!("Error while the container {:?}", err);
		return Err(err);
	}

	Ok(())
}
