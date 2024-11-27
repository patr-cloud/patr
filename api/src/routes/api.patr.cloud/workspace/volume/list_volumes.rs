use axum::http::StatusCode;
use models::{api::workspace::volume::*, utils::TotalCountHeader};

use crate::prelude::*;

pub async fn list_volumes(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListVolumesInWorkspacePath { workspace_id },
				query: Paginated {
					data: (),
					count,
					page,
				},
				headers:
					ListVolumesInWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListVolumesInWorkspaceRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, ListVolumesInWorkspaceRequest>,
) -> Result<AppResponse<ListVolumesInWorkspaceRequest>, ErrorType> {
	trace!("Listing volumes in workspace ID: `{workspace_id}`");

	let mut total_count = 0;
	let volumes = query!(
		r#"
		SELECT
			deployment_volume.*,
			(
				SELECT
					deployment_id
				FROM
					deployment_volume_mount
				WHERE
					volume_id = deployment_volume.id
			) AS deployment_id,
			COUNT(*) OVER() AS "total_count!"
		FROM
			deployment_volume
		JOIN
			resource
		ON
			deployment_volume.id = resource.id
		WHERE
			resource.owner_id = $1
		ORDER BY
			resource.created DESC
		LIMIT $2
		OFFSET $3;
		"#,
		workspace_id as _,
		count as i32,
		(page * count) as i32
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		total_count = row.total_count;
		WithId::new(
			row.id,
			DeploymentVolume {
				name: row.name,
				size: row.volume_size.unsigned_abs().into(),
				deployment_id: row.deployment_id.map(Into::into),
			},
		)
	})
	.collect::<Vec<_>>();

	AppResponse::builder()
		.body(ListVolumesInWorkspaceResponse { volumes })
		.headers(ListVolumesInWorkspaceResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
