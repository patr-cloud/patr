use api_models::utils::Uuid;
use chrono::{DateTime, Duration, TimeZone, Utc};
use redis::{
	aio::MultiplexedConnection as RedisConnection,
	AsyncCommands,
	RedisError,
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
fn get_key_for_user_api_token_data(token_id: &Uuid) -> String {
	format!("api-token-data:{}", token_id)
}
fn get_key_for_add_card_payment_intent_id(workspace_id: &Uuid) -> String {
	format!("workspace-add-card-payment-id:{}", workspace_id.as_str())
}
fn get_key_for_user_access_token_data(login_id: &Uuid) -> String {
	format!("{}.permission", login_id)
}

/// returns last set revocation timestamp (in millis) for the given user
pub async fn get_token_revoked_timestamp_for_user(
	redis_conn: &mut RedisConnection,
	user_id: &Uuid,
) -> Result<Option<DateTime<Utc>>, RedisError> {
	let timestamp: Option<i64> =
		redis_conn.get(get_key_for_user_revocation(user_id)).await?;
	Ok(timestamp.map(|timestamp| Utc.timestamp_millis_opt(timestamp).unwrap()))
}

/// returns last set revocation timestamp (in millis) for the given login
pub async fn get_token_revoked_timestamp_for_login(
	redis_conn: &mut RedisConnection,
	login_id: &Uuid,
) -> Result<Option<DateTime<Utc>>, RedisError> {
	let timestamp: Option<i64> = redis_conn
		.get(get_key_for_login_revocation(login_id))
		.await?;
	Ok(timestamp.map(|timestamp| Utc.timestamp_millis_opt(timestamp).unwrap()))
}

/// returns last set revocation timestamp (in millis) for the given
/// workspace
pub async fn get_token_revoked_timestamp_for_workspace(
	redis_conn: &mut RedisConnection,
	workspace_id: &Uuid,
) -> Result<Option<DateTime<Utc>>, RedisError> {
	let timestamp: Option<i64> = redis_conn
		.get(get_key_for_workspace_revocation(workspace_id))
		.await?;
	Ok(timestamp.map(|timestamp| Utc.timestamp_millis_opt(timestamp).unwrap()))
}

/// returns last set revocation timestamp (in millis) for global tokens
pub async fn get_global_token_revoked_timestamp(
	redis_conn: &mut RedisConnection,
) -> Result<Option<DateTime<Utc>>, RedisError> {
	let timestamp: Option<i64> =
		redis_conn.get(get_key_for_global_revocation()).await?;
	Ok(timestamp.map(|timestamp| Utc.timestamp_millis_opt(timestamp).unwrap()))
}

/// if ttl_in_secs is None, then key will live forever
pub async fn revoke_user_tokens_created_before_timestamp(
	redis_conn: &mut RedisConnection,
	user_id: &Uuid,
	timestamp: &DateTime<Utc>,
	ttl: Option<&Duration>,
) -> Result<(), RedisError> {
	let key = get_key_for_user_revocation(user_id);
	if let Some(ttl) = ttl {
		redis_conn
			.set_ex(
				key,
				timestamp.timestamp_millis(),
				ttl.num_seconds() as usize,
			)
			.await
	} else {
		redis_conn.set(key, timestamp.timestamp_millis()).await
	}
}

/// if ttl_in_secs is None, then key will live forever
pub async fn revoke_login_tokens_created_before_timestamp(
	redis_conn: &mut RedisConnection,
	login_id: &Uuid,
	timestamp: &DateTime<Utc>,
	ttl: Option<&Duration>,
) -> Result<(), RedisError> {
	let key = get_key_for_login_revocation(login_id);
	if let Some(ttl) = ttl {
		redis_conn
			.set_ex(
				key,
				timestamp.timestamp_millis(),
				ttl.num_seconds() as usize,
			)
			.await
	} else {
		redis_conn.set(key, timestamp.timestamp_millis()).await
	}
}

/// if ttl_in_secs is None, then key will live forever
pub async fn revoke_workspace_tokens_created_before_timestamp(
	redis_conn: &mut RedisConnection,
	workspace_id: &Uuid,
	timestamp: &DateTime<Utc>,
	ttl: Option<&Duration>,
) -> Result<(), RedisError> {
	let key = get_key_for_workspace_revocation(workspace_id);
	if let Some(ttl) = ttl {
		redis_conn
			.set_ex(
				key,
				timestamp.timestamp_millis(),
				ttl.num_seconds() as usize,
			)
			.await
	} else {
		redis_conn.set(key, timestamp.timestamp_millis()).await
	}
}

/// if ttl_in_secs is None, then key will live forever
#[allow(dead_code)]
pub async fn revoke_global_tokens_created_before_timestamp(
	redis_conn: &mut RedisConnection,
	timestamp: &DateTime<Utc>,
	ttl: Option<&Duration>,
) -> Result<(), RedisError> {
	let key = get_key_for_global_revocation();
	if let Some(ttl) = ttl {
		redis_conn
			.set_ex(
				key,
				timestamp.timestamp_millis(),
				ttl.num_seconds() as usize,
			)
			.await
	} else {
		redis_conn.set(key, timestamp.timestamp_millis()).await
	}
}

pub async fn get_user_api_token_data(
	redis_conn: &mut RedisConnection,
	token_id: &Uuid,
) -> Result<Option<String>, RedisError> {
	let token_data: Option<String> = redis_conn
		.get(get_key_for_user_api_token_data(token_id))
		.await?;
	Ok(token_data)
}

pub async fn set_user_api_token_data(
	redis_conn: &mut RedisConnection,
	token_id: &Uuid,
	token_data: &str,
	ttl: Option<&Duration>,
) -> Result<(), RedisError> {
	let key = get_key_for_user_api_token_data(token_id);
	if let Some(ttl) = ttl {
		redis_conn
			.set_ex(key, token_data, ttl.num_seconds() as usize)
			.await
	} else {
		redis_conn.set(key, token_data).await
	}
}

pub async fn delete_user_api_token_data(
	redis_conn: &mut RedisConnection,
	token_id: &Uuid,
) -> Result<(), RedisError> {
	redis_conn
		.del(get_key_for_user_api_token_data(token_id))
		.await
}

pub async fn set_add_card_payment_intent_id(
	redis_conn: &mut RedisConnection,
	workspace_id: &Uuid,
	payment_intent_id: &str,
) -> Result<(), RedisError> {
	let key = get_key_for_add_card_payment_intent_id(workspace_id);
	redis_conn.set(key, payment_intent_id).await
}

pub async fn get_add_card_payment_intent_id(
	redis_conn: &mut RedisConnection,
	workspace_id: &Uuid,
) -> Result<Option<String>, RedisError> {
	let payment_intent_id: Option<String> = redis_conn
		.get(get_key_for_add_card_payment_intent_id(workspace_id))
		.await?;
	Ok(payment_intent_id)
}

pub async fn get_user_access_token_data(
	redis_conn: &mut RedisConnection,
	login_id: &Uuid,
) -> Result<Option<String>, RedisError> {
	let token_data: Option<String> = redis_conn
		.get(get_key_for_user_access_token_data(login_id))
		.await?;
	Ok(token_data)
}

pub async fn set_user_access_token_data(
	redis_conn: &mut RedisConnection,
	login_id: &Uuid,
	token_data: &str,
	ttl: Option<&Duration>,
) -> Result<(), RedisError> {
	let key = get_key_for_user_access_token_data(login_id);
	if let Some(ttl) = ttl {
		redis_conn
			.set_ex(key, token_data, ttl.num_seconds() as usize)
			.await
	} else {
		redis_conn.set(key, token_data).await
	}
}

pub async fn delete_user_acess_token_data(
	redis_conn: &mut RedisConnection,
	login_id: &Uuid,
) -> Result<(), RedisError> {
	redis_conn
		.del(get_key_for_user_access_token_data(login_id))
		.await
}
