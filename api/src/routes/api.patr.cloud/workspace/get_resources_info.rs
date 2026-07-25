use std::{collections::HashMap, str::FromStr};

use axum::http::StatusCode;
use models::{api::workspace::*, rbac::ResourceType};

use crate::prelude::*;

/// The handler to resolve a batch of resource IDs into their names and resource
/// types. Resource names are not stored centrally — each resource type keeps
/// its name in its own table — so this joins the central `resource` table
/// against every name-bearing type table and coalesces the name.
///
/// The response contains one entry per requested ID. IDs that don't resolve to
/// a live resource in this workspace (deleted, or belonging to another
/// workspace) come back with `null` fields, as do resources whose type has no
/// name (e.g. a managed URL).
pub async fn get_resources_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetResourcesInfoPath { workspace_id },
				query: (),
				headers:
					GetResourcesInfoRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetResourcesInfoRequestProcessed { resource_ids },
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, GetResourcesInfoRequest>,
) -> Result<AppResponse<GetResourcesInfoRequest>, ErrorType> {
	info!(
		"Resolving {} resource ID(s) in workspace `{workspace_id}`",
		resource_ids.len()
	);

	let ids = resource_ids.iter().copied().collect::<Vec<_>>();

	let resolved = query!(
		r#"
		SELECT
			resource.id AS "id: Uuid",
			resource_type.name AS "resource_type!",
			COALESCE(
				deployment.name::TEXT,
				container_registry_repository.name,
				static_site.name::TEXT,
				deployment_volume.name,
				managed_database.name::TEXT,
				secret.name::TEXT,
				workspace_domain.name,
				runner.name
			) AS "name?"
		FROM
			resource
		INNER JOIN
			resource_type
			ON resource_type.id = resource.resource_type_id
		LEFT JOIN
			deployment
			ON deployment.id = resource.id
		LEFT JOIN
			container_registry_repository
			ON container_registry_repository.id = resource.id
		LEFT JOIN
			static_site
			ON static_site.id = resource.id
		LEFT JOIN
			deployment_volume
			ON deployment_volume.id = resource.id
		LEFT JOIN
			managed_database
			ON managed_database.id = resource.id
		LEFT JOIN
			secret
			ON secret.id = resource.id
		LEFT JOIN
			workspace_domain
			ON workspace_domain.id = resource.id
		LEFT JOIN
			runner
			ON runner.id = resource.id
		WHERE
			resource.owner_id = $1 AND
			resource.id = ANY($2::UUID[]) AND
			resource.deleted IS NULL;
		"#,
		workspace_id as _,
		ids as _,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		(
			row.id,
			(row.name, ResourceType::from_str(&row.resource_type).ok()),
		)
	})
	.collect::<HashMap<Uuid, (Option<String>, Option<ResourceType>)>>();

	let resources = resource_ids
		.into_iter()
		.map(|id| {
			let (name, resource_type) = resolved.get(&id).cloned().unwrap_or((None, None));
			WithId::new(
				id,
				ResourceInfo {
					name,
					resource_type,
				},
			)
		})
		.collect();

	AppResponse::builder()
		.body(GetResourcesInfoResponse { resources })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
