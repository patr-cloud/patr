use models::api::workspace::{
	managed_url::{ManagedUrl, ManagedUrlType},
	runner::StreamRunnerDataForWorkspaceServerMsg,
};

use crate::prelude::*;

/// Loads what a runner should know about a managed URL now: its current state
/// if it's live, in the workspace, and proxies to a live deployment on this
/// runner, or that it's gone otherwise.
pub(super) async fn load_event(
	connection: &mut DatabaseConnection,
	workspace_id: Uuid,
	runner_id: Uuid,
	managed_url_id: Uuid,
) -> Result<StreamRunnerDataForWorkspaceServerMsg, ErrorType> {
	let Some(row) = query!(
		r#"
		SELECT
			managed_url.sub_domain,
			managed_url.domain_id,
			managed_url.path,
			managed_url.deployment_id AS "deployment_id!",
			managed_url.port AS "port!",
			managed_url_custom_hostname.is_active
		FROM
			managed_url
		INNER JOIN
			managed_url_custom_hostname
		ON
			managed_url.sub_domain = managed_url_custom_hostname.sub_domain AND
			managed_url.domain_id = managed_url_custom_hostname.domain_id
		INNER JOIN
			deployment
		ON
			deployment.id = managed_url.deployment_id
		WHERE
			managed_url.id = $1 AND
			managed_url.workspace_id = $2 AND
			managed_url.deleted IS NULL AND
			managed_url.url_type = 'proxy_to_deployment' AND
			deployment.runner = $3 AND
			deployment.deleted IS NULL;
		"#,
		managed_url_id,
		workspace_id,
		runner_id,
	)
	.fetch_optional(&mut *connection)
	.await?
	else {
		return Ok(StreamRunnerDataForWorkspaceServerMsg::ManagedUrlDeleted { id: managed_url_id });
	};

	Ok(StreamRunnerDataForWorkspaceServerMsg::ManagedUrlUpdated {
		managed_url: WithId::new(
			managed_url_id,
			ManagedUrl {
				sub_domain: row.sub_domain,
				domain_id: row.domain_id,
				path: row.path,
				url_type: ManagedUrlType::ProxyDeployment {
					deployment_id: row.deployment_id,
					port: row.port as u16,
				},
				is_active: row.is_active,
			},
		),
	})
}
