use models::api::workspace::runner::StreamRunnerDataForWorkspaceServerMsg;
use preprocess::Preprocessable;
use tokio::sync::mpsc::UnboundedSender;
use typed_builder::TypedBuilder;

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
	/// The channel to notify the runner of changes to a particular resource.
	pub runner_changes_sender: UnboundedSender<StreamRunnerDataForWorkspaceServerMsg>,
	/// The application configuration.
	pub config: RunnerSettings<E::Settings>,
}

impl<E> Clone for AppState<E>
where
	E: RunnerExecutor,
{
	fn clone(&self) -> Self {
		Self {
			database: self.database.clone(),
			runner_changes_sender: self.runner_changes_sender.clone(),
			config: self.config.clone(),
		}
	}
}

/// This struct represents a preprocessed request to the API.
///
/// It contains the path, query, headers and preprocessed body of the request.
/// This struct provides a builder API to make it easier to construct requests.
#[derive(TypedBuilder)]
pub struct ProcessedApiRequest<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	/// The path of the request. This is the part of the URL after the domain
	/// and port.
	pub path: E::RequestPath,
	/// The query of the request. This is the part of the URL after the `?`.
	pub query: E::RequestQuery,
	/// The headers of the request.
	pub headers: E::RequestHeaders,
	/// The body of the request. This is the actual data that was sent by the
	/// client. Can be either JSON or Websockets.
	pub body: <E::RequestBody as Preprocessable>::Processed,
}

impl<E> TryFrom<ApiRequest<E>> for ProcessedApiRequest<E>
where
	E: ApiEndpoint,
	<E::RequestBody as Preprocessable>::Processed: Send,
{
	type Error = preprocess::Error;

	fn try_from(value: ApiRequest<E>) -> Result<Self, Self::Error> {
		let ApiRequest {
			path,
			query,
			headers,
			body,
		} = value;
		Ok(ProcessedApiRequest {
			path,
			query,
			headers,
			body: body.preprocess()?,
		})
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
	/// The channel to notify the runner of changes to a particular resource.
	pub runner_changes_sender: UnboundedSender<StreamRunnerDataForWorkspaceServerMsg>,
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
	/// The channel to notify the runner of changes to a particular resource.
	pub runner_changes_sender: UnboundedSender<StreamRunnerDataForWorkspaceServerMsg>,
	/// The Application Config.
	pub config: RunnerSettings<()>,
}
