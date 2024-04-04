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

pub async fn get_deployment_log(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetDeploymentLogPath {
					workspace_id,
					deployment_id,
				},
				query: GetDeploymentLogQuery { end_time, limit },
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, GetDeploymentLogRequest>,
) -> Result<AppResponse<GetDeploymentLogRequest>, ErrorType> {
	info!("Starting: Get deployment logs");

	// LOGIC

	AppResponse::builder()
		.body(GetDeploymentLogResponse { logs: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()

}