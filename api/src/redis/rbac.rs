use api_models::utils::Uuid;
use redis::{
	aio::MultiplexedConnection as RedisConnection,
	AsyncCommands,
	RedisError,
};

use crate::{
	models::AccessTokenData,
	service::get_access_token_expiry,
	utils::get_current_time_millis,
};

fn get_key_for_user_revocation(user_id: &Uuid) -> String {
	format!("token-revoked:user:{}", user_id.as_str())
}
fn get_key_for_login_revocation(login_id: &Uuid) -> String {
	format!("token-revoked:login:{}", login_id.as_str())
}
fn get_key_for_workspace_revocation(workspace_id: &Uuid) -> String {
	format!("token-revoked:workspace:{}", workspace_id.as_str())
}
fn get_key_for_global_revocation() -> String {
	"token-revoked:global".to_string()
}

/// returns last set revocation timestamp (in millis) for the given user
async fn get_token_revoked_timestamp_for_user(
	redis_conn: &mut RedisConnection,
	user_id: &Uuid,
) -> Result<Option<u64>, RedisError> {
	redis_conn.get(get_key_for_user_revocation(user_id)).await
}

/// returns last set revocation timestamp (in millis) for the given login
async fn get_token_revoked_timestamp_for_login(
	redis_conn: &mut RedisConnection,
	login_id: &Uuid,
) -> Result<Option<u64>, RedisError> {
	redis_conn.get(get_key_for_login_revocation(login_id)).await
}

/// returns last set revocation timestamp (in millis) for the given
/// workspace
async fn get_token_revoked_timestamp_for_workspace(
	redis_conn: &mut RedisConnection,
	workspace_id: &Uuid,
) -> Result<Option<u64>, RedisError> {
	redis_conn
		.get(get_key_for_workspace_revocation(workspace_id))
		.await
}

/// returns last set revocation timestamp (in millis) for global tokens
async fn get_global_token_revoked_timestamp(
	redis_conn: &mut RedisConnection,
) -> Result<Option<u64>, RedisError> {
	redis_conn.get(get_key_for_global_revocation()).await
}

/// if ttl_in_secs is None, then key will live forever
async fn revoke_user_tokens_created_before_timestamp(
	redis_conn: &mut RedisConnection,
	user_id: &Uuid,
	timestamp_in_millis: u64,
	ttl_in_secs: Option<usize>,
) -> Result<(), RedisError> {
	let key = get_key_for_user_revocation(user_id);
	if let Some(ttl) = ttl_in_secs {
		redis_conn.set_ex(key, timestamp_in_millis, ttl).await
	} else {
		redis_conn.set(key, timestamp_in_millis).await
	}
}

/// if ttl_in_secs is None, then key will live forever
async fn revoke_login_tokens_created_before_timestamp(
	redis_conn: &mut RedisConnection,
	login_id: &Uuid,
	timestamp_in_millis: u64,
	ttl_in_secs: Option<usize>,
) -> Result<(), RedisError> {
	let key = get_key_for_login_revocation(login_id);
	if let Some(ttl) = ttl_in_secs {
		redis_conn.set_ex(key, timestamp_in_millis, ttl).await
	} else {
		redis_conn.set(key, timestamp_in_millis).await
	}
}

/// if ttl_in_secs is None, then key will live forever
async fn revoke_workspace_tokens_created_before_timestamp(
	redis_conn: &mut RedisConnection,
	workspace_id: &Uuid,
	timestamp_in_millis: u64,
	ttl_in_secs: Option<usize>,
) -> Result<(), RedisError> {
	let key = get_key_for_workspace_revocation(workspace_id);
	if let Some(ttl) = ttl_in_secs {
		redis_conn.set_ex(key, timestamp_in_millis, ttl).await
	} else {
		redis_conn.set(key, timestamp_in_millis).await
	}
}

/// if ttl_in_secs is None, then key will live forever
async fn revoke_global_tokens_created_before_timestamp(
	redis_conn: &mut RedisConnection,
	timestamp_in_millis: u64,
	ttl_in_secs: Option<usize>,
) -> Result<(), RedisError> {
	let key = get_key_for_global_revocation();
	if let Some(ttl) = ttl_in_secs {
		redis_conn.set_ex(key, timestamp_in_millis, ttl).await
	} else {
		redis_conn.set(key, timestamp_in_millis).await
	}
}

pub async fn revoke_access_tokens_for_user(
	redis_conn: &mut RedisConnection,
	user_id: &Uuid,
) -> Result<(), RedisError> {
	let ttl = (get_access_token_expiry() / 60) as usize + 120; // 120 seconds buffer time
	revoke_user_tokens_created_before_timestamp(
		redis_conn,
		user_id,
		get_current_time_millis(),
		Some(ttl),
	)
	.await
}

pub async fn is_access_token_revoked(
	redis_conn: &mut RedisConnection,
	token: &AccessTokenData,
) -> Result<bool, RedisError> {
	// check user revocation
	let revoked_timestamp =
		get_token_revoked_timestamp_for_user(redis_conn, &token.user.id)
			.await?;
	if matches!(revoked_timestamp, Some(revoked_timestamp) if token.iat < revoked_timestamp)
	{
		return Ok(true);
	}

	// check login revocation
	let revoked_timestamp =
		get_token_revoked_timestamp_for_login(redis_conn, &token.login_id)
			.await?;
	if matches!(revoked_timestamp, Some(revoked_timestamp) if token.iat < revoked_timestamp)
	{
		return Ok(true);
	}

	// check workspace revocation
	for workspace_id in token.workspaces.keys() {
		let revoked_timestamp =
			get_token_revoked_timestamp_for_workspace(redis_conn, workspace_id)
				.await?;
		if matches!(revoked_timestamp, Some(revoked_timestamp) if token.iat < revoked_timestamp)
		{
			return Ok(true);
		}
	}

	// check global revocation
	let revoked_timestamp =
		get_global_token_revoked_timestamp(redis_conn).await?;
	if matches!(revoked_timestamp, Some(revoked_timestamp) if token.iat < revoked_timestamp)
	{
		return Ok(true);
	}

	// all checks are passed, hence token has not revoked
	Ok(false)
}
