use std::{cmp::Ordering, collections::BTreeMap};

use axum::{http::StatusCode, Router};
use futures::sink::With;
use models::{
	api::{
		workspace::{
			container_registry::{ContainerRepository, ContainerRepositoryTagInfo},
			infrastructure::{
				deployment::*,
				managed_url::{DbManagedUrlType, ManagedUrl, ManagedUrlType},
			},
			region::{Region, RegionStatus},
		},
		WithId,
	},
	utils::StringifiedU16,
	ApiRequest,
	ErrorType,
};
use sqlx::query_as;
use time::OffsetDateTime;

use crate::{models::deployment::MACHINE_TYPES, prelude::*, utils::validator};

pub async fn list_deployment_history(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListDeploymentHistoryPath {
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
	}: AuthenticatedAppRequest<'_, ListDeploymentHistoryRequest>,
) -> Result<AppResponse<ListDeploymentHistoryRequest>, ErrorType> {
	info!("Starting: List deployment history");

	// Check if deployment exists
	query!(
		r#"
		SELECT
			id
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
	.ok_or(ErrorType::ResourceDoesNotExist);

	let deploys = query!(
		r#"
		SELECT 
			image_digest,
			created
		FROM
			deployment_deploy_history
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|deploy| DeploymentDeployHistory {
		image_digest: deploy.image_digest,
		created: deploy.created,
	})
	.collect();

	AppResponse::builder()
		.body(ListDeploymentHistoryResponse { deploys })
		.headers(ListDeploymentHistoryResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}