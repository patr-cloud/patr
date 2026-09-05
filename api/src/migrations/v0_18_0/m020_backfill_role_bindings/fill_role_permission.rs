use crate::prelude::*;

/// A role's flat permission list, from the legacy type rows.
pub(super) async fn fill_role_permission(
	connection: &mut DatabaseConnection,
) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		INSERT INTO
			role_permission(
				role_id,
				permission_id
			)
		SELECT DISTINCT
			t.role_id,
			t.permission_id
		FROM
			role_resource_permissions_type t
		ON CONFLICT
			(role_id, permission_id)
		DO NOTHING;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
