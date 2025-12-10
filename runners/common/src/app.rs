use models::api::workspace::deployment::DeploymentStatus;
use preprocess::Preprocessable;
use tokio::sync::{mpsc::UnboundedSender, watch};

use crate::prelude::*;

/// Represents a deployment status update event that is sent from the
/// resource executor task to the main runner.
#[derive(Debug, Clone)]
pub enum ExecutorStatusUpdate {
	/// A deployment's status has been updated.
	DeploymentStatusUpdated {
		/// The ID of the deployment that was updated.
		deployment_id: Uuid,
		/// The new status of the deployment.
		status: DeploymentStatus,
	},
}

/// The global state of the application.
/// This will contain the database connection and other configuration.
#[derive(Debug)]
pub struct AppState<E>
where
	E: RunnerExecutor,
{
	/// The database connection.
	pub database: sqlx::Pool<DatabaseType>,
	/// The application configuration.
	pub config: RunnerSettings<E::Settings>,
	/// The initialized state of the runner. This will be used to create new
	/// instances of the runner.
	pub runner_state: E::InitializedState,
	/// Channel sender for deployment status updates. When a resource executor
	/// task updates a deployment's status, it sends a signal through this
	/// channel so the main runner can react to the change.
	pub task_status_sender: UnboundedSender<ExecutorStatusUpdate>,
	/// Channel sender for reloading nginx configuration. When a resource
	/// executor task updates a deployment that requires nginx configuration
	/// change, it sends a signal through this channel so the nginx server can
	/// reload its configuration.
	pub nginx_reload_sender: watch::Sender<()>,
}

impl<E> Clone for AppState<E>
where
	E: RunnerExecutor,
{
	fn clone(&self) -> Self {
		Self {
			database: self.database.clone(),
			config: self.config.clone(),
			runner_state: self.runner_state.clone(),
			task_status_sender: self.task_status_sender.clone(),
			nginx_reload_sender: self.nginx_reload_sender.clone(),
		}
	}
}

/// A request object that is passed through the tower layers and services for
/// endpoints that do not require authentication
pub struct UnprocessedAppRequest<'a, E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The Endpoint that the request is being made for. This would ideally be
	/// parsed to have all the data needed to process a request
	pub request: ApiRequest<E>,
	/// The database transaction for the request. In case the request returns
	/// an Error, this transaction will be automatically rolled back.
	pub database: &'a mut DatabaseTransaction,
	/// The Application Config.
	pub config: RunnerSettings<()>,
}

/// A request object that is passed through the tower layers and services for
/// endpoints that do not require authentication
pub struct AppRequest<'a, E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The Endpoint that the request is being made for. This would ideally be
	/// parsed and preprocessed to have all the data needed to process a request
	pub request: ProcessedApiRequest<E>,
	/// The database transaction for the request. In case the request returns
	/// an Error, this transaction will be automatically rolled back.
	pub database: &'a mut DatabaseTransaction,
	/// The Application Config.
	pub config: RunnerSettings<()>,
}
