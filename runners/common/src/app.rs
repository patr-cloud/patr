use preprocess::Preprocessable;

use crate::prelude::*;

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
