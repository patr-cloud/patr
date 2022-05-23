use api_models::utils::Uuid;

use crate::{query, Database};

pub async fn add_ci_info(
	connection: &mut <Database as sqlx::Database>::Connection,
	drone_username: &str,
	workspace_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE 
			workspace
		SET 
			drone_username = $1
		WHERE id = $2
		"#,
		drone_username,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
