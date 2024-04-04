use std::{cmp::Ordering, collections::BTreeMap};

use axum::{http::StatusCode, Router};
use futures::sink::With;
use models::{
	api::workspace::infrastructure::deployment::*,
	ErrorType,
};
use sqlx::query_as;
use time::OffsetDateTime;

use crate::{models::deployment::MACHINE_TYPES, prelude::*, utils::validator};

pub async fn get_deployment_metric(
	AuthenticatedAppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, GetDeploymentMetricRequest>,
) -> Result<AppResponse<GetDeploymentMetricRequest>, ErrorType> {
	info!("Starting: Get deployment metrics");

	// LOGIC

	AppResponse::builder()
		.body(GetDeploymentMetricResponse { metrics: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
