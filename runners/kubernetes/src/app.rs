use kube::Client;
use models::prelude::*;
use thiserror::Error;

/// Represents the state of the application. This is used to share information
/// across the entire application, such as the API token, the region ID, etc.
pub struct AppState {
	/// The API token used to authenticate with the Patr API.
	pub patr_token: String,
	/// The region ID of the cluster.
	pub region_id: Uuid,
	/// The workspace ID of the cluster.
	pub workspace_id: Uuid,
	/// The kubernetes client used to communicate with the cluster.
	pub client: Client,
}

impl AppState {
	/// Tries to create a new `AppState` from the environment variables. If the
	/// environment variables are not set in release mode, it will panic. In
	/// debug mode, it will use the default values.
	pub async fn try_default() -> Self {
		let patr_token = std::env::var("PATR_TOKEN");
		let region_id = std::env::var("REGION_ID");
		let workspace_id = std::env::var("WORKSPACE_ID");

		let patr_token = if cfg!(debug_assertions) {
			patr_token.unwrap_or_default()
		} else {
			patr_token.expect(concat!(
				"could not find environment variable PATR_TOKEN. ",
				"Please generate a token for your cluster at ",
				"the Patr dashboard and use it as an environment variable."
			))
		};
		let region_id = Uuid::parse_str(
			&if cfg!(debug_assertions) {
				region_id.unwrap_or_default()
			} else {
				region_id.expect(concat!(
					"could not find environment variable REGION_ID. ",
					"Please set the region ID of your cluster as an environment variable."
				))
			},
		);
		let region_id = if cfg!(debug_assertions) {
			region_id.unwrap_or_default()
		} else {
			region_id.expect("malformed region ID")
		};

		let workspace_id = Uuid::parse_str(
			&if cfg!(debug_assertions) {
				workspace_id.unwrap_or_default()
			} else {
				workspace_id.expect(concat!(
					"could not find environment variable WORKSPACE_ID. ",
					"Please set the region ID of your cluster as an environment variable."
				))
			},
		);
		let workspace_id = if cfg!(debug_assertions) {
			workspace_id.unwrap_or_default()
		} else {
			workspace_id.expect("malformed workspace ID")
		};

		let client = Client::try_default()
			.await
			.expect("Failed to get kubernetes client details");

		Self {
			patr_token,
			region_id,
			workspace_id,
			client,
		}
	}
}

#[derive(Error, Debug)]
pub enum AppError {
	#[error("error while communicating with the Kubernetes API: {0}")]
	Kubernetes(#[from] kube::Error),
	#[error("error while communicating with the Patr API: {0}")]
	Patr(ErrorType),
	#[error("internal error: {0}")]
	InternalError(String),
}

impl From<ErrorType> for AppError {
	fn from(err: ErrorType) -> Self {
		Self::Patr(err)
	}
}
