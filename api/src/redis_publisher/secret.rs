use models::api::workspace::runner::StreamRunnerDataForWorkspaceServerMsg;

use crate::prelude::*;

/// Loads what a runner should know about a secret now: when its value last
/// changed. Nothing if it's been deleted or isn't in the workspace.
pub(super) async fn load_event(
	connection: &mut DatabaseConnection,
	workspace_id: Uuid,
	secret_id: Uuid,
) -> Result<Option<StreamRunnerDataForWorkspaceServerMsg>, ErrorType> {
	Ok(query!(
		r#"
		SELECT
			last_updated
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
	.fetch_optional(&mut *connection)
	.await?
	.map(
		|secret| StreamRunnerDataForWorkspaceServerMsg::SecretUpdated {
			id: secret_id,
			last_updated: secret.last_updated,
		},
	))
}
