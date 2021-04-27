use shiplift::{Docker, Error};

/// function to start a container
pub async fn start_container(
	docker: &Docker,
	container_id: &str,
) -> Result<(), Error> {
	docker.containers().get(container_id).start().await
}

/// stop container
pub async fn stop_container(
	docker: &Docker,
	container_name: &str,
) -> Result<(), Error> {
	docker.containers().get(container_name).stop(None).await
}

/// delete container
/// this function will stop and delete a container.
pub async fn delete_container(
	docker: &Docker,
	container_name: &str,
) -> Result<(), Error> {
	// stop container
	let container_stop_result = stop_container(docker, container_name).await;

	let container_delete_result =
		docker.containers().get(container_name).delete().await;

	// if container is not deleted, then start back the contaienr
	if container_delete_result.is_err() {
		let container_start_result =
			docker.containers().get(container_name).start().await;
		if let Err(err) = container_start_result {
			// throw error
			return Err(err);
		}
	}

	if let Err(err) = container_stop_result.and(container_delete_result) {
		return Err(err);
	}
	Ok(())
}
