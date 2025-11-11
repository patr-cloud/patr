//! Background cleanup tasks for the container registry.
//!
//! This module provides functionality to clean up expired upload sessions
//! and their associated S3 multipart uploads.

use s3::Bucket;
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};

use crate::{
	prelude::*,
	routes::registry_patr_cloud::utils::s3::abort_multipart_upload,
};

/// The default threshold for considering an upload session expired (in hours).
/// Sessions that haven't been updated in this time will be cleaned up.
const DEFAULT_EXPIRY_THRESHOLD_HOURS: i64 = 24;

/// Represents an expired upload session that needs cleanup.
#[derive(Debug)]
struct ExpiredSession {
	/// The session ID
	id: models::utils::Uuid,
	/// The AWS S3 multipart upload ID
	aws_session_id: Option<String>,
	/// The blob digest (used as S3 key)
	blob_digest: Option<String>,
}

/// Run the upload session cleanup task.
///
/// This function queries for expired upload sessions, aborts their S3 multipart
/// uploads, and deletes them from the database.
///
/// # Arguments
///
/// * `database` - The database connection pool
/// * `s3_bucket` - The S3 bucket for blob storage
/// * `expiry_threshold_hours` - Optional custom expiry threshold in hours
///
/// # Returns
///
/// The number of sessions cleaned up
///
/// # Errors
///
/// Returns an error if database or S3 operations fail
#[tracing::instrument(skip(database, s3_bucket))]
pub async fn cleanup_expired_sessions(
	database: &PgPool,
	s3_bucket: &Bucket,
	expiry_threshold_hours: Option<i64>,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
	let threshold_hours = expiry_threshold_hours.unwrap_or(DEFAULT_EXPIRY_THRESHOLD_HOURS);
	let cutoff_time = OffsetDateTime::now_utc() - Duration::hours(threshold_hours);

	info!(
		threshold_hours = threshold_hours,
		cutoff_time = %cutoff_time,
		"Starting upload session cleanup"
	);

	// Query for expired sessions
	let expired_sessions = query_expired_sessions(database, cutoff_time).await?;
	let session_count = expired_sessions.len();

	if session_count == 0 {
		debug!("No expired sessions found");
		return Ok(0);
	}

	info!(
		session_count = session_count,
		"Found expired sessions to clean up"
	);

	let mut cleaned_count = 0;
	let mut failed_count = 0;

	// Process each expired session
	for session in expired_sessions {
		match cleanup_session(database, s3_bucket, session).await {
			Ok(()) => {
				cleaned_count += 1;
			}
			Err(e) => {
				failed_count += 1;
				error!(
					error = %e,
					"Failed to clean up session"
				);
			}
		}
	}

	info!(
		cleaned_count = cleaned_count,
		failed_count = failed_count,
		"Upload session cleanup completed"
	);

	Ok(cleaned_count)
}

/// Query the database for expired upload sessions.
///
/// # Arguments
///
/// * `database` - The database connection pool
/// * `cutoff_time` - Sessions updated before this time are considered expired
///
/// # Returns
///
/// A vector of expired sessions
async fn query_expired_sessions(
	database: &PgPool,
	cutoff_time: OffsetDateTime,
) -> Result<Vec<ExpiredSession>, sqlx::Error> {
	let rows = sqlx::query!(
		r#"
		SELECT id, aws_session_id, blob_digest
		FROM container_registry_session
		WHERE updated_at < $1
		"#,
		cutoff_time
	)
	.fetch_all(database)
	.await?;

	let sessions: Vec<ExpiredSession> = rows
		.into_iter()
		.map(|row| ExpiredSession {
			id: row.id.into(),
			aws_session_id: row.aws_session_id,
			blob_digest: row.blob_digest,
		})
		.collect();

	Ok(sessions)
}

/// Clean up a single expired session.
///
/// This function:
/// 1. Aborts the S3 multipart upload if one exists
/// 2. Deletes the session from the database
///
/// # Arguments
///
/// * `database` - The database connection pool
/// * `s3_bucket` - The S3 bucket for blob storage
/// * `session` - The expired session to clean up
async fn cleanup_session(
	database: &PgPool,
	s3_bucket: &Bucket,
	session: ExpiredSession,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	debug!(
		session_id = %session.id,
		"Cleaning up expired session"
	);

	// Abort S3 multipart upload if it exists
	if let (Some(aws_session_id), Some(blob_digest)) = (&session.aws_session_id, &session.blob_digest) {
		let s3_key = format!("blobs/{}", blob_digest);
		
		match abort_multipart_upload(s3_bucket, &s3_key, aws_session_id).await {
			Ok(()) => {
				debug!(
					session_id = %session.id,
					s3_key = %s3_key,
					upload_id = %aws_session_id,
					"Aborted S3 multipart upload"
				);
			}
			Err(e) => {
				// Log the error but continue with database cleanup
				// S3 lifecycle policies will eventually clean up incomplete uploads
				warn!(
					session_id = %session.id,
					s3_key = %s3_key,
					upload_id = %aws_session_id,
					error = %e,
					"Failed to abort S3 multipart upload, will be cleaned by lifecycle policy"
				);
			}
		}
	}

	// Delete the session from the database
	sqlx::query!(
		r#"
		DELETE FROM container_registry_session
		WHERE id = $1
		"#,
		session.id as models::utils::Uuid
	)
	.execute(database)
	.await?;

	info!(
		session_id = %session.id,
		"Successfully cleaned up expired session"
	);

	Ok(())
}

/// Start a background task that periodically cleans up expired upload sessions.
///
/// This function spawns a tokio task that runs the cleanup process at regular
/// intervals. The task will continue running until the program exits.
///
/// # Arguments
///
/// * `database` - The database connection pool
/// * `s3_bucket` - The S3 bucket for blob storage
/// * `interval_minutes` - How often to run the cleanup (in minutes)
/// * `expiry_threshold_hours` - Optional custom expiry threshold in hours
///
/// # Returns
///
/// A JoinHandle for the background task
pub fn start_cleanup_task(
	database: PgPool,
	s3_bucket: Box<Bucket>,
	interval_minutes: u64,
	expiry_threshold_hours: Option<i64>,
) -> tokio::task::JoinHandle<()> {
	info!(
		interval_minutes = interval_minutes,
		expiry_threshold_hours = ?expiry_threshold_hours,
		"Starting upload session cleanup background task"
	);

	tokio::spawn(async move {
		let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_minutes * 60));

		loop {
			interval.tick().await;

			debug!("Running scheduled upload session cleanup");

			match cleanup_expired_sessions(&database, &s3_bucket, expiry_threshold_hours).await {
				Ok(count) => {
					if count > 0 {
						info!(cleaned_count = count, "Scheduled cleanup completed");
					}
				}
				Err(e) => {
					error!(error = %e, "Scheduled cleanup failed");
				}
			}
		}
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_default_expiry_threshold() {
		assert_eq!(DEFAULT_EXPIRY_THRESHOLD_HOURS, 24);
	}

	#[test]
	fn test_expired_session_structure() {
		let session = ExpiredSession {
			id: Uuid::new_v4(),
			aws_session_id: Some("test-upload-id".to_string()),
			blob_digest: Some("sha256:abc123".to_string()),
		};

		assert!(session.aws_session_id.is_some());
		assert!(session.blob_digest.is_some());
	}
}
