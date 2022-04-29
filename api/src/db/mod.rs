mod initializer;
mod meta_data;
mod rbac;
mod user;
mod workspace;

use sqlx::{pool::PoolOptions, Connection, Database as Db, Pool};

pub use self::{initializer::*, meta_data::*, rbac::*, user::*, workspace::*};
use crate::{query, utils::settings::Settings, Database};

pub async fn create_database_connection(
	config: &Settings,
) -> Result<Pool<Database>, sqlx::Error> {
	log::trace!("Creating database connection pool...");
	PoolOptions::<Database>::new()
		.max_connections(config.database.connection_limit)
		.connect_with(
			<<Database as Db>::Connection as Connection>::Options::new()
				.username(&config.database.user)
				.password(&config.database.password)
				.host(&config.database.host)
				.port(config.database.port)
				.database(&config.database.database),
		)
		.await
}

pub async fn begin_deferred_constraints(
	connection: &mut <Database as sqlx::Database>::Connection,
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

pub async fn end_deferred_constraints(
	connection: &mut <Database as sqlx::Database>::Connection,
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
