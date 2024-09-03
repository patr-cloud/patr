use crate::prelude::*;

/// TODO: FromRef Implementation and axum stuff
#[derive(Clone)]
/// The global state of the application.
/// This will contain the database connection and other configuration.
pub struct AppState<'a, E>
where
	E: RunnerExecutor,
{
	/// The database connection.
	pub database: sqlx::Pool<DatabaseType>,
	/// The application configuration.
	pub config: RunnerSettings<E::Settings<'a>>,
}
