mod initializer;
mod meta_data;
mod rbac;
mod user;
mod workspace;

use std::ops::{Deref, DerefMut};

use sqlx::{pool::PoolOptions, Connection, Database as Db, Pool};

pub use self::{initializer::*, meta_data::*, rbac::*, user::*, workspace::*};
use crate::prelude::*;

pub struct DatabaseError(sqlx::Error);

impl From<sqlx::Error> for DatabaseError {
	fn from(error: sqlx::Error) -> Self {
		Self(error)
	}
}

impl From<DatabaseError> for Error {
	fn from(DatabaseError(error): DatabaseError) -> Self {
		match error {
			sqlx::Error::Configuration(_) => todo!(),
			sqlx::Error::Database(_) => todo!(),
			sqlx::Error::Io(_) => todo!(),
			sqlx::Error::Tls(_) => todo!(),
			sqlx::Error::Protocol(_) => todo!(),
			sqlx::Error::RowNotFound => todo!(),
			sqlx::Error::TypeNotFound { type_name } => todo!(),
			sqlx::Error::ColumnIndexOutOfBounds { index, len } => todo!(),
			sqlx::Error::ColumnNotFound(_) => todo!(),
			sqlx::Error::ColumnDecode { index, source } => todo!(),
			sqlx::Error::Decode(_) => todo!(),
			sqlx::Error::PoolTimedOut => todo!(),
			sqlx::Error::PoolClosed => todo!(),
			sqlx::Error::WorkerCrashed => todo!(),
			sqlx::Error::Migrate(_) => todo!(),
			err => {
				Self::new(ErrorType::InternalServerError(anyhow::anyhow!(err)))
			}
		}
		todo!("check constraints and convert error")
	}
}

impl Deref for DatabaseError {
	type Target = sqlx::Error;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for DatabaseError {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

pub type DatabaseResult<T> = Result<T, DatabaseError>;

pub async fn create_database_connection(
	config: &Config,
) -> DatabaseResult<Pool<Database>> {
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
) -> DatabaseResult<()> {
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
) -> DatabaseResult<()> {
	query!(
		r#"
		SET CONSTRAINTS ALL IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
