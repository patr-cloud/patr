use std::{cmp::Ordering, collections::BTreeMap};

use axum::{http::StatusCode, Router};
use futures::sink::With;
use models::{
	api::{
		workspace::infrastructure::deployment::*,
		WithId,
	},
	ErrorType,
};
use sqlx::query_as;
use time::OffsetDateTime;

use crate::{models::deployment::MACHINE_TYPES, prelude::*, utils::validator};

pub async fn list_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListDeploymentPath { workspace_id },
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, ListDeploymentRequest>,
) -> Result<AppResponse<ListDeploymentRequest>, ErrorType> {
	info!("Starting: List deployments");

	let deployments = query!(
		r#"
		SELECT
			id,
			name,
			registry,
			repository_id,
			image_name,
			image_tag,
			status as "status: DeploymentStatus",
			region,
			machine_type,
			current_live_digest
		FROM
			deployment
		WHERE
			workspace_id = $1 AND
			status != 'deleted';
		"#,
		workspace_id as _
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|deployment| {
		WithId::new(
			deployment.id.into(),
			Deployment {
				name: deployment.name,
				registry: if deployment.registry == PatrRegistry.to_string() {
					DeploymentRegistry::PatrRegistry {
						registry: PatrRegistry,
						repository_id: deployment.repository_id.unwrap().into(),
					}
				} else {
					DeploymentRegistry::ExternalRegistry {
						registry: deployment.registry,
						image_name: deployment.image_name.unwrap().into(),
					}
				},
				image_tag: deployment.image_tag,
				status: deployment.status,
				region: deployment.region.into(),
				machine_type: deployment.machine_type.into(),
				current_live_digest: deployment.current_live_digest,
			},
		)
	})
	.collect();

	todo!("Filter out deployments that are not supposed to be viewed");

	AppResponse::builder()
		.body(ListDeploymentResponse { deployments })
		.headers(ListDeploymentResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
