use std::collections::BTreeMap;

use models::{api::user::*, rbac::WorkspacePermission, utils::TotalCountHeader};
use reqwest::StatusCode;

use crate::prelude::*;

pub async fn list_api_tokens(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListApiTokensPath,
				query: Paginated {
					data: (),
					count,
					page,
				},
				headers:
					ListApiTokensRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListApiTokensRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data,
		config: _,
	}: AuthenticatedAppRequest<'_, ListApiTokensRequest>,
) -> Result<AppResponse<ListApiTokensRequest>, ErrorType> {
	trace!("Listing API tokens for user: {}", user_data.id);

	let mut total_count = 0;
	let tokens = query!(
		r#"
		SELECT
			token_id,
			name,
			token_nbf,
			token_exp,
			allowed_ips,
			created,
			COUNT(*) OVER() AS "total_count!"
		FROM
			user_api_token
		WHERE
			user_id = $1 AND
			revoked IS NULL
		ORDER BY
			created DESC
		LIMIT $2
		OFFSET $3;
		"#,
		user_data.id as _,
		count as i32,
		(count * page) as i32,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		total_count = row.total_count;
		WithId::new(
			row.token_id,
			UserApiToken {
				name: row.name,
				permissions: BTreeMap::<Uuid, WorkspacePermission>::new(),
				token_nbf: row.token_nbf,
				token_exp: row.token_exp,
				allowed_ips: row.allowed_ips,
				created: row.created,
			},
		)
	})
	.collect();

	AppResponse::builder()
		.body(ListApiTokensResponse { tokens })
		.headers(ListApiTokensResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
