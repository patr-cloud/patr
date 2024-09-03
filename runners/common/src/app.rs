use crate::prelude::*;

/// The global state of the application.
/// This will contain the database connection and other configuration.
#[derive(Clone)]
pub struct AppState<E>
where
	E: RunnerExecutor,
{
	/// The database connection.
	pub database: sqlx::Pool<DatabaseType>,
	/// The application configuration.
	pub config: RunnerSettings<E::Settings>,
}
