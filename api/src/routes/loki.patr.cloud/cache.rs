use rustis::{client::Client as RedisClient, commands::StringCommands as _};

use crate::prelude::*;

/// Look up the workspace that owns a runner, using Redis cache with DB
/// fallback. Returns `None` if the runner doesn't exist or is deleted.
pub(super) async fn get_workspace_for_runner(
	database: &mut DatabaseConnection,
	redis: &mut RedisClient,
	runner_id: &Uuid,
) -> Result<Option<Uuid>, ErrorType> {
	let cache_key = redis::keys::workspace_id_for_runner(runner_id);

	// Check cache first
	if let Some(value) = redis.get::<Option<String>>(&cache_key).await? {
		if value.is_empty() {
			// Negative cache: runner was previously looked up and not found
			return Ok(None);
		}
		if let Ok(workspace_id) = value.parse::<Uuid>() {
			return Ok(Some(workspace_id));
		}
	}

	// Cache miss — query the database
	let result = query!(
		r#"
		SELECT
			workspace_id AS "workspace_id: Uuid"
		FROM
			runner
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		runner_id as _
	)
	.fetch_optional(&mut *database)
	.await?
	.map(|row| row.workspace_id);

	// Cache the result (empty string for not-found)
	let cache_value = result.map(|id| id.to_string()).unwrap_or_default();
	let _ = redis
		.setex(&cache_key, super::CACHE_TTL.as_secs(), &cache_value)
		.await;

	Ok(result)
}

/// Look up the runner that owns a deployment, using Redis cache with DB
/// fallback. Returns `None` if the deployment doesn't exist or is deleted.
pub(super) async fn get_runner_for_deployment(
	database: &mut DatabaseConnection,
	redis: &mut RedisClient,
	deployment_id: &Uuid,
) -> Result<Option<Uuid>, ErrorType> {
	let cache_key = redis::keys::runner_id_for_deployment(deployment_id);

	// Check cache first
	if let Some(value) = redis.get::<Option<String>>(&cache_key).await? {
		if value.is_empty() {
			return Ok(None);
		}
		if let Ok(runner_id) = value.parse::<Uuid>() {
			return Ok(Some(runner_id));
		}
	}

	// Cache miss — query the database
	let result = query!(
		r#"
		SELECT
			runner AS "runner: Uuid"
		FROM
			deployment
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		deployment_id as _
	)
	.fetch_optional(&mut *database)
	.await?
	.map(|row| row.runner);

	let cache_value = result.map(|id| id.to_string()).unwrap_or_default();
	let _ = redis
		.setex(&cache_key, super::CACHE_TTL.as_secs(), &cache_value)
		.await;

	Ok(result)
}
