use crate::prelude::*;

/// One `workspace_actor` row per distinct `(user, workspace)` membership pair.
pub(super) async fn add_actors_to_table(
	connection: &mut DatabaseConnection,
) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		INSERT INTO
			workspace_actor(
				id,
				workspace_id,
				actor_type,
				user_id
			)
		SELECT
			gen_random_uuid(),
			wu.workspace_id,
			'user',
			wu.user_id
		FROM
			(
				SELECT DISTINCT
					user_id,
					workspace_id
				FROM
					workspace_user
			) wu
		ON CONFLICT
			(user_id, workspace_id)
		DO NOTHING;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
