use api_models::utils::Uuid;

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
			"user"
		WHERE
			id NOT IN (
				'543de4f5808f4a99b2ee96dbdf9afff7',
				'b4560e9530904195a0999c6d26aa9c29'
			);
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
		INNER JOIN
			workspace
		ON
			deployment.workspace_id = workspace.id
		WHERE
			deployment.status != 'deleted' AND
			workspace.super_admin_id NOT IN (
				'543de4f5808f4a99b2ee96dbdf9afff7',
				'b4560e9530904195a0999c6d26aa9c29'
			);
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
		INNER JOIN
			workspace
		ON
			managed_url.workspace_id = workspace.id
		WHERE
			url_type = 'proxy_to_deployment' AND
			workspace.super_admin_id NOT IN (
				'543de4f5808f4a99b2ee96dbdf9afff7',
				'b4560e9530904195a0999c6d26aa9c29'
			);
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
		INNER JOIN
			workspace
		ON
			deployment.workspace_id = workspace.id
		WHERE
			deployment.status = 'deleted' AND
			workspace.super_admin_id NOT IN (
				'543de4f5808f4a99b2ee96dbdf9afff7',
				'b4560e9530904195a0999c6d26aa9c29'
			);
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
		INNER JOIN
			workspace
		ON
			managed_database.workspace_id = workspace.id
		WHERE
			status != 'deleted' AND
			workspace.super_admin_id NOT IN (
				'543de4f5808f4a99b2ee96dbdf9afff7',
				'b4560e9530904195a0999c6d26aa9c29'
			);
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
		INNER JOIN
			workspace
		ON
			managed_database.workspace_id = workspace.id
		WHERE
			status = 'deleted' AND
			workspace.super_admin_id NOT IN (
				'543de4f5808f4a99b2ee96dbdf9afff7',
				'b4560e9530904195a0999c6d26aa9c29'
			);
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
		INNER JOIN
			workspace
		ON
			deployment_static_site.workspace_id = workspace.id
		WHERE
			status != 'deleted' AND
			workspace.super_admin_id NOT IN (
				'543de4f5808f4a99b2ee96dbdf9afff7',
				'b4560e9530904195a0999c6d26aa9c29'
			);
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
		INNER JOIN
			workspace
		ON
			managed_url.workspace_id = workspace.id
		WHERE
			url_type = 'proxy_to_static_site' AND
			workspace.super_admin_id NOT IN (
				'543de4f5808f4a99b2ee96dbdf9afff7',
				'b4560e9530904195a0999c6d26aa9c29'
			);
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
		INNER JOIN
			workspace
		ON
			deployment_static_site.workspace_id = workspace.id
		WHERE 
			status = 'deleted' AND
			workspace.super_admin_id NOT IN (
				'543de4f5808f4a99b2ee96dbdf9afff7',
				'b4560e9530904195a0999c6d26aa9c29'
			);
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

pub async fn get_deployment_count_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<u64, sqlx::Error> {
	let count = query!(
		r#"
		SELECT
			COUNT(*) as "count!"
		FROM
			deployment
		WHERE
			workspace_id = $1;
		"#,
		workspace_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.map(|row| row.count)
	.unwrap_or(0);

	Ok(count as u64)
}

pub async fn get_database_count_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<u64, sqlx::Error> {
	let count = query!(
		r#"
		SELECT
			COUNT(*) as "count!"
		FROM
			managed_database
		WHERE
			workspace_id = $1;
		"#,
		workspace_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.map(|row| row.count)
	.unwrap_or(0);

	Ok(count as u64)
}

pub async fn get_static_site_count_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<u64, sqlx::Error> {
	let count = query!(
		r#"
		SELECT
			COUNT(*) as "count!"
		FROM
			deployment_static_site
		WHERE
			workspace_id = $1;
		"#,
		workspace_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.map(|row| row.count)
	.unwrap_or(0);

	Ok(count as u64)
}

pub async fn _get_docker_repositories_size_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<u64, sqlx::Error> {
	let count = query!(
		r#"
		SELECT
			COALESCE(SUM(size), 0) as "count!: i64"
		FROM
			docker_registry_repository_manifest
		INNER JOIN
			docker_registry_repository
		ON
			docker_registry_repository.id = docker_registry_repository_manifest.repository_id
		WHERE
			docker_registry_repository.workspace_id = $1;
		"#,
		workspace_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.map(|row| row.count)
	.unwrap_or(0);

	Ok(count as u64)
}

pub async fn get_managed_url_count_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<u64, sqlx::Error> {
	let count = query!(
		r#"
		SELECT
			COUNT(*) as "count!"
		FROM
			managed_url
		WHERE
			workspace_id = $1;
		"#,
		workspace_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.map(|row| row.count)
	.unwrap_or(0);

	Ok(count as u64)
}

pub async fn get_secret_count_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<u64, sqlx::Error> {
	let count = query!(
		r#"
		SELECT
			COUNT(*) as "count!"
		FROM
			secret
		WHERE
			workspace_id = $1;
		"#,
		workspace_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.map(|row| row.count)
	.unwrap_or(0);

	Ok(count as u64)
}

pub async fn get_domain_count_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<u64, sqlx::Error> {
	let count = query!(
		r#"
		SELECT
			COUNT(*) as "count!"
		FROM
			workspace_domain
		INNER JOIN
			resource
		ON
			resource.id = workspace_domain.id
		WHERE
			resource.owner_id = $1;
		"#,
		workspace_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.map(|row| row.count)
	.unwrap_or(0);

	Ok(count as u64)
}
