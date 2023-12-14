use sqlx::{pool::PoolOptions, Pool};

use crate::{prelude::*, utils::config::DatabaseConfig};

pub(super) mod initializer;
pub(super) mod meta_data;
pub(super) mod rbac;
pub(super) mod user;
pub(super) mod workspace;

/// The diesel database schema and types for querying
#[allow(missing_docs)]
pub mod schema;

pub use self::initializer::initialize;
pub(super) use self::{meta_data::*, rbac::*, user::*, workspace::*};

/// Connects to the database based on a config. Not much to say here.
#[instrument(skip(config))]
pub async fn connect(config: &DatabaseConfig) -> Pool<DatabaseType> {
	info!("Connecting to database `{}:{}`", config.host, config.port);
	PoolOptions::<DatabaseType>::new()
		.max_connections(config.connection_limit)
		.connect_with(
			<DatabaseConnection as sqlx::Connection>::Options::new()
				.username(config.user.as_str())
				.password(config.password.as_str())
				.host(config.host.as_str())
				.port(config.port)
				.database(config.database.as_str()),
		)
		.await
		.expect("Failed to connect to database")
}

/// Sets all constraints to deferred for this particular connection
pub async fn begin_deferred_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		SET CONSTRAINTS ALL DEFERRED;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Ends all deferred constraints for this connection.
/// Note: This can cause errors if the constraints are violated.
pub async fn end_deferred_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		SET CONSTRAINTS ALL IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
