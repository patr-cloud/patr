use crate::prelude::*;

/// The global state of the application.
/// This will contain the database connection and other configuration.
pub struct AppState<E>
where
	E: RunnerExecutor,
{
	/// The database connection.
	pub database: sqlx::Pool<DatabaseType>,
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
			config: self.config.clone(),
		}
	}
}
