use std::{cmp::Ordering, collections::BTreeMap};

use axum::{http::StatusCode, Router};
use futures::sink::With;
use models::{api::workspace::deployment::*, ErrorType};
use sqlx::query_as;
use time::OffsetDateTime;

use crate::prelude::*;

/// Delete a deployment
///
/// #Parameters
/// - `workspace_id`: The workspace ID
/// - `deployment_id`: The deployment ID
///
/// #Returns
/// - `OK`: The deployment was deleted
pub async fn delete_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteDeploymentPath {
					workspace_id,
					deployment_id,
				},
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, DeleteDeploymentRequest>,
) -> Result<AppResponse<DeleteDeploymentRequest>, ErrorType> {
	info!("Starting: Delete deployment");

	// Check if deployment exists
	let deployment = query!(
		r#"
		SELECT
			runner
		FROM
			deployment
		WHERE
			id = $1 AND
			status != 'deleted';
		"#,
		deployment_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	// Check if deployment is using managed URLs
	let managed_urls_exists = query!(
		r#"
		SELECT
			id
		FROM
			managed_url
		WHERE
			managed_url.deployment_id = $1 AND
			managed_url.workspace_id = $2 AND
			managed_url.url_type = 'proxy_to_deployment' AND
			managed_url.deleted IS NULL;
		"#,
		deployment_id as _,
		workspace_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	if managed_urls_exists {
		return Err(ErrorType::ResourceInUse);
	}

	// Check if deployment is on patr region to stop usage tracking
	if todo!("Check if deployment on patr region") {
		todo!("Stop deployment usage history");

		let volumes: BTreeMap<String, DeploymentVolume> = query!(
			r#"
			SELECT
				id,
				name,
				volume_size,
				volume_mount_path
			FROM
				deployment_volume
			WHERE
				deployment_id = $1 AND
				deleted IS NULL;
			"#,
			deployment_id as _,
		)
		.fetch_all(&mut **database)
		.await?
		.into_iter()
		.map(|volume| {
			(
				volume.name,
				DeploymentVolume {
					path: volume.volume_mount_path,
					size: volume.volume_size as u16,
				},
			)
		})
		.collect();

		for volume in volumes {
			todo!("Stop volume usage history");
		}
	}

	// Mark deployment deleted in database
	query!(
		r#"
		UPDATE
			deployment
		SET
			deleted = $2,
			status = 'deleted'
		WHERE
			id = $1;
		"#,
		deployment_id as _,
		OffsetDateTime::now_utc()
	)
	.execute(&mut **database)
	.await?;

	todo!("Audit log");

	AppResponse::builder()
		.body(DeleteDeploymentResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
