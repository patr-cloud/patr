use crate::{query, Database};

pub async fn get_sign_up_count(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<u64, sqlx::Error> {
	let count = query!(
		r#"
		SELECT
			COUNT(*) as "count!"
		FROM
			user_to_sign_up;
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.map(|row| row.count)
	.unwrap_or(0);

	Ok(count as u64)
}

pub async fn get_join_count(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<u64, sqlx::Error> {
	let count = query!(
		r#"
		SELECT
			COUNT(*) as "count!"
		FROM
			"user";
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.map(|row| row.count)
	.unwrap_or(0);

	Ok(count as u64)
}

pub async fn get_created_deployment_count(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<u64, sqlx::Error> {
	let count = query!(
		r#"
		SELECT
			COUNT(*) as "count!"
		FROM
			deployment
		WHERE
			status != 'deleted';
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.map(|row| row.count)
	.unwrap_or(0);

	Ok(count as u64)
}

pub async fn get_deployment_domain_count(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<u64, sqlx::Error> {
	let count = query!(
		r#"
		SELECT
			COUNT(*) as "count!"
		FROM
			managed_url
		WHERE
			url_type = 'proxy_to_deployment';
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.map(|row| row.count)
	.unwrap_or(0);

	Ok(count as u64)
}

pub async fn get_deleted_deployment_count(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<u64, sqlx::Error> {
	let count = query!(
		r#"
		SELECT
			COUNT(*) "count!"
		FROM
			deployment
		WHERE
			status = 'deleted';
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.map(|row| row.count)
	.unwrap_or(0);

	Ok(count as u64)
}

pub async fn get_created_database_count(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<u64, sqlx::Error> {
	let count = query!(
		r#"
		SELECT
			COUNT(*) as "count!"
		FROM
			managed_database
		WHERE
			status != 'deleted';
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.map(|row| row.count)
	.unwrap_or(0);

	Ok(count as u64)
}

pub async fn get_deleted_database_count(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<u64, sqlx::Error> {
	let count = query!(
		r#"
		SELECT
			COUNT(*) as "count!"
		FROM
			managed_database
		WHERE
			status = 'deleted';
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.map(|row| row.count)
	.unwrap_or(0);

	Ok(count as u64)
}

pub async fn get_created_static_site_count(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<u64, sqlx::Error> {
	let count = query!(
		r#"
		SELECT
			COUNT(*) as "count!"
		FROM
			deployment_static_site
		WHERE
			status != 'deleted';
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.map(|row| row.count)
	.unwrap_or(0);

	Ok(count as u64)
}

pub async fn get_static_site_domain_count(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<u64, sqlx::Error> {
	let count = query!(
		r#"
		SELECT 
			COUNT(*) as "count!"
		FROM
			managed_url
		WHERE
			url_type = 'proxy_to_static_site';
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.map(|row| row.count)
	.unwrap_or(0);

	Ok(count as u64)
}

pub async fn get_deleted_static_site_count(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<u64, sqlx::Error> {
	let count = query!(
		r#"
		SELECT
			COUNT(*) as "count!"
		FROM 
			deployment_static_site
		WHERE 
			status = 'deleted';
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.map(|row| row.count)
	.unwrap_or(0);

	Ok(count as u64)
}
