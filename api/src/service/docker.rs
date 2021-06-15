use shiplift::{Docker, Error};

/// # Description
/// This function is used to start a container
///
/// # Arguments
/// * `docker` - An instance of [`Docker`]
/// * `container_id` - A string which contains id of the container
///
/// # Returns
/// This function returns `Result<(), Error>` which is either the container will
/// start successfully or an error will be returned
///
/// [`Docker`]: Docker
pub async fn start_container(
	docker: &Docker,
	container_id: &str,
) -> Result<(), Error> {
	docker.containers().get(container_id).start().await
}

/// # Description
/// This function is used to stop the docker container
///
/// # Arguments
/// * `docker` - An instance of [`Docker`]
/// * `container_name` - A string which contains name of the container
///
/// # Returns
/// This function returns `Result<(), Error>` which is either the container will
/// stop successfully or an error will be returned
/// [`Docker`]: Docker
pub async fn stop_container(
	docker: &Docker,
	container_name: &str,
) -> Result<(), Error> {
	docker.containers().get(container_name).stop(None).await
}

/// # Description
/// This function is used to stop and delete the docker container
///
/// # Arguments
/// * `docker` - An instance of [`Docker`]
/// * `container_name` - A string which contains name of the container
///
/// # Returns
/// This function returns `Result<(), Error>` which is either the container will
/// stop and delete successfully or an error will be returned
/// [`Docker`]: Docker
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
