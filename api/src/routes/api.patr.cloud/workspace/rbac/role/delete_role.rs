use axum::http::StatusCode;
use models::api::workspace::rbac::role::*;
use rustis::commands::StringCommands;
use time::OffsetDateTime;

use crate::prelude::*;

pub async fn delete_role(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteRolePath {
					workspace_id,
					role_id,
				},
				query: (),
				headers: DeleteRoleRequestHeaders {
					authorization: _,
					user_agent: _,
				},
				body: DeleteRoleRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, DeleteRoleRequest>,
) -> Result<AppResponse<DeleteRoleRequest>, ErrorType> {
	info!("Deleting role: {} in workspace: {}", role_id, workspace_id);

	query!(
		r#"
        DELETE FROM
            role_resource_permissions_include
        WHERE
            role_id = $1;
        "#,
		role_id as _
	)
	.execute(&mut **database)
	.await?;

	trace!("Deleted all the included permissions");

	query!(
		r#"
        DELETE FROM
            role_resource_permissions_exclude
        WHERE
            role_id = $1;
        "#,
		role_id as _
	)
	.execute(&mut **database)
	.await?;

	trace!("Deleted all the excluded permissions");

	query!(
		r#"
        DELETE FROM
            role_resource_permissions_type
        WHERE
            role_id = $1;
        "#,
		role_id as _
	)
	.execute(&mut **database)
	.await?;

	trace!("Deleted all the permission types");

	query!(
		r#"
        DELETE FROM
            role
        WHERE
            id = $1;
        "#,
		role_id as _
	)
	.execute(&mut **database)
	.await?;

	trace!("Deleted the role");

	redis
		.setex(
			redis::keys::workspace_id_revocation_timestamp(&workspace_id),
			constants::CACHED_PERMISSIONS_VALIDITY.whole_seconds() as u64,
			OffsetDateTime::now_utc().unix_timestamp(),
		)
		.await
		.inspect_err(|err| {
			error!("Error setting the revocation timestamp: `{}`", err);
		})?;

	trace!("Revocation timestamp set");

	AppResponse::builder()
		.body(DeleteRoleResponse)
		.headers(())
		.status_code(StatusCode::RESET_CONTENT)
		.build()
		.into_result()
}
