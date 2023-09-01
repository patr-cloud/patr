#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! This project generates a controller that can be installed in the user's
//! Kubernetes cluster. Each controller will be responsible for managing the
//! respective cluster. The controller will periodically check with the API and
//! make sure that the cluster's state is up to date with the API's state.

#[tokio::main]
async fn main() -> Result<(), kube::Error> {
	Ok(())
}
