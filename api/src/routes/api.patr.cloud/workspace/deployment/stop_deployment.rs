use std::{cmp::Ordering, collections::BTreeMap};

use axum::{http::StatusCode, Router};
use futures::sink::With;
use models::{
	api::workspace::deployment::*,
	ErrorType,
};
use sqlx::query_as;
use time::OffsetDateTime;

use crate::prelude::*;

/// Stop deployment
/// 
/// #Parameters
/// - `workspace_id`: The workspace ID
/// - `deployment_id`: The deployment ID
/// 
/// #Returns
/// - `OK`: The deployment was stopped
/// 
pub async fn stop_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: StopDeploymentPath {
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
	}: AuthenticatedAppRequest<'_, StopDeploymentRequest>,
) -> Result<AppResponse<StopDeploymentRequest>, ErrorType> {
	info!("Starting: Stop deployment");

	// Updating deployment status
	query!(
		r#"
		UPDATE
			deployment
		SET
			status = $1
		WHERE
			id = $2;
		"#,
		DeploymentStatus::Stopped as _,
		deployment_id as _
	)
	.execute(&mut **database)
	.await?;

	todo!("Stop deployment usage history");
	todo!("Audit log");

	AppResponse::builder()
		.body(StopDeploymentResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}