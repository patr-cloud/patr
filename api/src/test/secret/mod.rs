#[cfg(test)]
mod tests {
	use chrono::Utc;

	use crate::{
		db::*,
		models::rbac,
		test::{deinit_test, user_constaints},
		utils::Error,
	};

	#[tokio::test]
	async fn secret_test() -> Result<(), Error> {
		let app = user_constaints().await?;
		let mut connection = app.database.acquire().await?;
		let workspace_id =
			get_workspace_by_name(&mut connection, "test-workspace")
				.await?
				.unwrap()
				.id;
		let resource_id = generate_new_resource_id(&mut connection).await?;
		// Create secret
		create_resource(
			&mut connection,
			&resource_id,
			rbac::RESOURCE_TYPES
				.get()
				.unwrap()
				.get(rbac::resource_types::SECRET)
				.unwrap(),
			&workspace_id,
			&Utc::now(),
		)
		.await?;
		let secret_name = "test";
		create_new_secret_in_workspace(
			&mut connection,
			&resource_id,
			secret_name,
			&workspace_id,
		)
		.await?;
		//Test
		let secret_output =
			get_all_secrets_in_workspace(&mut connection, &workspace_id)
				.await?;
		assert_eq!(secret_name, secret_output[0].name);
		assert_eq!(workspace_id, secret_output[0].workspace_id);
		assert_eq!(None, secret_output[0].deployment_id);

		//Update secret
		let updated_secret_name = "new_test";
		update_secret_name(
			&mut connection,
			&secret_output[0].id,
			updated_secret_name,
		)
		.await?;
		// Test
		let updated_secret_output =
			get_all_secrets_in_workspace(&mut connection, &workspace_id)
				.await?;
		assert_eq!(
			updated_secret_name.to_string(),
			updated_secret_output[0].name
		);

		//Delete secret
		delete_secret(&mut connection, &secret_output[0].id, &Utc::now())
			.await?;
		//Test
		let deleted_secret_output =
			get_all_secrets_in_workspace(&mut connection, &workspace_id)
				.await?;
		assert_eq!(0, deleted_secret_output.len());

		deinit_test(app.config.database.database).await?;
		Ok(())
	}
}
