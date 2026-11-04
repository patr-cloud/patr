use axum::http::StatusCode;
use models::{api::workspace::secret::*, utils::TotalCountHeader};

use crate::prelude::*;

/// Lists the deployments using a secret, with the variables each one uses it as.
/// `secret::view` doesn't imply `deployment::view`, so only the deployments the
/// caller can view are returned.
pub async fn list_secret_deployments(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListSecretDeploymentsPath {
					workspace_id,
					secret_id,
				},
				query:
					ListResourceQueryProcessed {
						sort: _,
						search:
							SecretDeploymentSearchParams {
								name: name_filter,
								status: status_filter,
							},
						count,
						page,
						additional_query: (),
					},
				headers: _,
				body: ListSecretDeploymentsRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		actor_data,
		state: _,
	}: AuthenticatedAppRequest<'_, ListSecretDeploymentsRequest>,
) -> Result<AppResponse<ListSecretDeploymentsRequest>, ErrorType> {
	trace!("Listing deployments using secret ID: `{secret_id}`");

	// The authenticator only proves this is some resource in the workspace.
	query!(
		r#"
		SELECT
			id
		FROM
			secret
		WHERE
			id = $1 AND
			workspace_id = $2 AND
			deleted IS NULL;
		"#,
		secret_id,
		workspace_id,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let mut total_count = 0;
	let deployments = query!(
		r#"
		SELECT
			deployment.id,
			deployment.name AS "name: String",
			deployment.status,
			ARRAY_AGG(
				deployment_environment_variable.name
				ORDER BY deployment_environment_variable.name
			) AS "variables!: Vec<String>",
			COUNT(*) OVER() AS "total_count!"
		FROM
			deployment_environment_variable
		INNER JOIN
			deployment
		ON
			deployment.id = deployment_environment_variable.deployment_id
		INNER JOIN
			RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID($3, $4) AS resource
		ON
			deployment.id = resource.id
		WHERE
			deployment_environment_variable.secret_id = $1 AND
			deployment_environment_variable.workspace_id = $2 AND
			($5::TEXT IS NULL OR deployment.name ILIKE '%' || $5 || '%') AND
			(
				$6::DEPLOYMENT_STATUS[] IS NULL OR
				deployment.status = ANY($6)
			)
		GROUP BY
			deployment.id
		ORDER BY
			deployment.name
		LIMIT $7
		OFFSET $8;
		"#,
		secret_id,
		workspace_id,
		actor_data.login_id,
		Permission::Deployment(DeploymentPermission::View) as _,
		name_filter,
		status_filter as _,
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
			SecretDeployment {
				name: row.name,
				status: row.status,
				variables: row.variables,
			},
		)
	})
	.collect::<Vec<_>>();

	if page != 0 && total_count == 0 {
		return Err(ErrorType::PageOutOfBounds);
	}

	AppResponse::builder()
		.body(ListSecretDeploymentsResponse { deployments })
		.headers(ListSecretDeploymentsResponseHeaders {
			total_count: TotalCountHeader(total_count as _),
		})
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
