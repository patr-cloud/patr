use std::convert::TryInto;

use sqlx::{MySql, Transaction};

use crate::{models::db_mapping::OauthClientUrl, query, query_as};

pub async fn initialize_oauth_pre(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing Portus tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS oauth_client(
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(100) NOT NULL,
			secret_key BINARY(64) NOT NULL
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS oauth_client_url(
			id BINARY(16),
			url VARCHAR(500),
			PRIMARY KEY (id, url)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn oauth_insert_into_table_oauth_client_url(
	connection: &mut Transaction<'_, MySql>,
	redirect_url: String,
	id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			oauth_client_url
		VALUES
			(?,?);
		"#,
		id,
		redirect_url
	)
	.execute(connection)
	.await?;
	Ok(())
}
// query to enter data into db
pub async fn oauth_insert_into_table_oauth_client(
	connection: &mut Transaction<'_, MySql>,
	id: &[u8],
	name: &str,
	secret_key: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			oauth_client
		VALUES
			(?,?,?);
		"#,
		id,
		name,
		secret_key,
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn oauth_is_url_registered(
	connection: &mut Transaction<'_, MySql>,
	id: &[u8],
) -> Result<Option<String>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			url
		FROM
			oauth_client_url
		WHERE
			id = ?
		"#,
		id
	)
	.fetch_all(connection)
	.await?;

	// if rows.is_empty() {
	// 	return Ok(None);
	// }
	// Ok(rows.into_iter().map(|row| row.url).next())
	Ok(rows.into_iter().map(|row| row.url).next())
}

// pub async fn oauth_is_url_registered(
// 	connection: &mut Transaction<'_, MySql>,
// 	id: &[u8],
// ) -> Result<Vec<OauthClientUrl>, sqlx::Error> {
// 	query_as!(
// 		OauthClientUrl,
// 		r#"
// 		SELECT
// 			*
// 		FROM
// 			oauth_client_url
// 		WHERE
// 			id = ?
// 		"#,
// 		id
// 	)
// 	.fetch_all(connection)
// 	.await?;
// }
// query to check if redirect url exists in the database
// query to check if client exists in the database
