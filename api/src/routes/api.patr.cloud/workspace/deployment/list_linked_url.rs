use std::{cmp::Ordering, collections::BTreeMap};

use axum::{http::StatusCode, Router};
use futures::sink::With;
use models::{
	api::{
		workspace::
			infrastructure::{
				deployment::*,
				managed_url::{DbManagedUrlType, ManagedUrl, ManagedUrlType},
			}
		,
		WithId,
	},
	ErrorType,
};
use sqlx::query_as;
use time::OffsetDateTime;

use crate::{models::deployment::MACHINE_TYPES, prelude::*, utils::validator};

pub async fn list_linked_url(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListLinkedURLPath {
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
	}: AuthenticatedAppRequest<'_, ListLinkedURLRequest>,
) -> Result<AppResponse<ListLinkedURLRequest>, ErrorType> {
	info!("Starting: List linked URLs");

	let urls = query!(
		r#"
		SELECT
			id,
			sub_domain,
			domain_id,
			path,
			url_type as "url_type: DbManagedUrlType",
			is_configured,
			deployment_id,
			port,
			static_site_id,
			http_only,
			url,
			permanent_redirect
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
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|url| {
		WithId::new(
			url.id.into(),
			ManagedUrl {
				sub_domain: url.sub_domain,
				domain_id: url.domain_id.into(),
				path: url.path,
				url_type: match url.url_type {
					DbManagedUrlType::ProxyToDeployment => ManagedUrlType::ProxyDeployment {
						deployment_id: url.deployment_id.unwrap().into(),
						port: url.port.unwrap() as u16,
					},
					DbManagedUrlType::ProxyToStaticSite => ManagedUrlType::ProxyStaticSite {
						static_site_id: url.static_site_id.unwrap().into(),
					},
					DbManagedUrlType::ProxyUrl => ManagedUrlType::ProxyUrl {
						url: url.url.unwrap(),
						http_only: url.http_only.unwrap(),
					},
					DbManagedUrlType::Redirect => ManagedUrlType::Redirect {
						url: url.url.unwrap(),
						permanent_redirect: url.permanent_redirect.unwrap(),
						http_only: url.http_only.unwrap(),
					},
				},
				is_configured: url.is_configured,
			},
		)
	})
	.collect();

	AppResponse::builder()
		.body(ListLinkedURLResponse { urls })
		.headers(ListLinkedURLResponseHeaders {
			total_count: todo!(),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
