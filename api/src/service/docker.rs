// DOCKER SERVICE
use shiplift::{Docker, Error};

/// function to start a container
pub async fn start_container(
	docker: &Docker,
	container_id: &str,
) -> Result<(), Error> {
	docker.containers().get(&container_id).start().await
}

/// stop container
pub async fn stop_container(
	docker: &Docker,
	container_name: &str,
) -> Result<(), Error> {
	docker.containers().get(&container_name).stop(None).await
}

/// delete container
pub async fn delete_container(
	docker: &Docker,
	container_name: &str,
) -> Result<(), Error> {
	docker.containers().get(&container_name).delete().await
}
