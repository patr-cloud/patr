use std::collections::BTreeMap;

use models::{api::user::*, rbac::WorkspacePermission, utils::TotalCountHeader};
use reqwest::StatusCode;

use crate::prelude::*;

pub async fn list_api_tokens(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListApiTokensPath,
				query:
					ListResourceQuery {
						sort: sort_order,
						search:
							UserApiTokenSearchParams {
								name: name_filter,
								token_nbf: token_nbf_filter,
								token_exp: token_exp_filter,
								allowed_ips: allowed_ips_filter,
								created: created_filter,
							},
						count,
						page,
						additional_query: (),
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
		state: _,
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
			revoked IS NULL AND
			($2::TEXT IS NULL OR name ILIKE '%' || $2 || '%') AND
			($3::TIMESTAMPTZ IS NULL OR token_nbf >= $3) AND
			($4::TIMESTAMPTZ IS NULL OR token_nbf <= $4) AND
			($5::TIMESTAMPTZ IS NULL OR token_exp >= $5) AND
			($6::TIMESTAMPTZ IS NULL OR token_exp <= $6) AND
			($7::INET IS NULL OR $7::INET <<= ANY(allowed_ips)) AND
			($8::TIMESTAMPTZ IS NULL OR created >= $8) AND
			($9::TIMESTAMPTZ IS NULL OR created <= $9)
		ORDER BY
			created DESC
		LIMIT $10
		OFFSET $11;
		"#,
		user_data.id as _,
		name_filter,
		token_nbf_filter.as_ref().map(|token_nbf| token_nbf.start()) as _,
		token_nbf_filter.as_ref().map(|token_nbf| token_nbf.end()) as _,
		token_exp_filter.as_ref().map(|token_exp| token_exp.start()) as _,
		token_exp_filter.as_ref().map(|token_exp| token_exp.end()) as _,
		allowed_ips_filter,
		created_filter.as_ref().map(|created_at| created_at.start()) as _,
		created_filter.as_ref().map(|created_at| created_at.end()) as _,
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
