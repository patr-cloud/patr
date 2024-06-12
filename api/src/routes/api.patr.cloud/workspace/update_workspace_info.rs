use axum::http::StatusCode;
use models::api::workspace::*;

use crate::prelude::*;

/// The handler to update the information of a workspace. At the moment, only
/// the name can be updated. However, this will be expanded in the future. At
/// least one parameter must be provided for the update.
pub async fn update_workspace_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateWorkspaceInfoPath { workspace_id },
				query: (),
				headers:
					UpdateWorkspaceInfoRequestHeaders {
						authorization,
						user_agent,
					},
				body: UpdateWorkspaceInfoRequestProcessed { name },
			},
		database,
		redis,
		client_ip,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, UpdateWorkspaceInfoRequest>,
) -> Result<AppResponse<UpdateWorkspaceInfoRequest>, ErrorType> {
	info!("Updating information for workspace `{workspace_id}`");

	// If more parameters are added, add them here
	if name.is_none() {
		return Err(ErrorType::WrongParameters);
	}

	if let Some(ref name) = name {
		let available = super::is_name_available(AuthenticatedAppRequest {
			request: ProcessedApiRequest {
				path: IsWorkspaceNameAvailablePath,
				query: IsWorkspaceNameAvailableQuery {
					name: name.to_string(),
				},
				headers: IsWorkspaceNameAvailableRequestHeaders {
					authorization,
					user_agent,
				},
				body: IsWorkspaceNameAvailableRequestProcessed,
			},
			client_ip,
			config,
			database,
			redis,
			user_data,
		})
		.await?
		.body
		.available;

		if !available {
			return Err(ErrorType::WorkspaceNameAlreadyExists);
		}
	}

	query!(
		r#"
        UPDATE
            workspace
        SET
            name = COALESCE($1, name)
		WHERE
			id = $2;
        "#,
		name.as_deref(),
		&workspace_id as _,
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(UpdateWorkspaceInfoResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
