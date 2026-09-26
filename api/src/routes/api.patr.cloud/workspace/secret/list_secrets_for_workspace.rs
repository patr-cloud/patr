use axum::http::StatusCode;
use models::{api::workspace::secret::*, utils::TotalCountHeader};
use time::OffsetDateTime;

use crate::prelude::*;

/// Lists the secrets in a workspace. Only metadata (id and name) is returned —
/// the value lives in OpenBao and is never read here.
pub async fn list_secrets_for_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListSecretsForWorkspacePath { workspace_id },
				query:
					ListResourceQueryProcessed {
						sort: _,
						search:
							SecretSearchParams {
								name: name_filter,
								created: created_filter,
								last_updated: last_updated_filter,
							},
						count,
						page,
						additional_query: (),
					},
				headers: _,
				body: ListSecretsForWorkspaceRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, ListSecretsForWorkspaceRequest>,
) -> Result<AppResponse<ListSecretsForWorkspaceRequest>, ErrorType> {
	trace!("Listing secrets in workspace ID: `{workspace_id}`");

	let mut total_count = 0;
	let secrets = query!(
		r#"
		SELECT
			secret.id AS "id: Uuid",
			secret.name AS "name: String",
			resource.created AS "created: OffsetDateTime",
			secret.last_updated AS "last_updated: OffsetDateTime",
			COUNT(*) OVER() AS "total_count!"
		FROM
			secret
		INNER JOIN
			resource
		ON
			secret.id = resource.id
		WHERE
			secret.workspace_id = $1 AND
			secret.deleted IS NULL AND
			($2::TEXT IS NULL OR secret.name ILIKE '%' || $2 || '%') AND
			($3::TIMESTAMPTZ IS NULL OR resource.created >= $3) AND
			($4::TIMESTAMPTZ IS NULL OR resource.created <= $4) AND
			($5::TIMESTAMPTZ IS NULL OR secret.last_updated >= $5) AND
			($6::TIMESTAMPTZ IS NULL OR secret.last_updated <= $6)
		ORDER BY
			resource.created DESC
		LIMIT $7
		OFFSET $8;
		"#,
		workspace_id as _,
		name_filter,
		created_filter.as_ref().map(|created| created.start()) as _,
		created_filter.as_ref().map(|created| created.end()) as _,
		last_updated_filter
			.as_ref()
			.map(|last_updated| last_updated.start()) as _,
		last_updated_filter
			.as_ref()
			.map(|last_updated| last_updated.end()) as _,
		count as i32,
		(page * count) as i32,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		total_count = row.total_count;
		WithId::new(
			row.id,
			Secret {
				name: row.name,
				created: row.created,
				last_updated: row.last_updated,
			},
		)
	})
	.collect::<Vec<_>>();

	if page != 0 && total_count == 0 {
		return Err(ErrorType::PageOutOfBounds);
	}

	AppResponse::builder()
		.body(ListSecretsForWorkspaceResponse { secrets })
		.headers(ListSecretsForWorkspaceResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
