#[cfg(test)]
mod tests {
	use api_models::models::workspace::infrastructure::{
		deployment::DeploymentStatus,
		static_site::{CreateStaticSiteRequest, StaticSiteDetails},
	};
	use chrono::Utc;

	use crate::{
		db::*,
		models::rbac,
		test::{deinit_test, user_constaints},
		utils::Error,
	};

	#[tokio::test]
	async fn create_static_test() -> Result<(), Error> {
		let app = user_constaints().await?;
		let mut connection = app.database.acquire().await?;
		let workspace_id =
			get_workspace_by_name(&mut connection, "test-workspace")
				.await?
				.unwrap()
				.id;

		let static_site_id = generate_new_resource_id(&mut connection).await?;
		let static_site = CreateStaticSiteRequest {
			workspace_id: workspace_id.clone(),
			name: "test-site".to_string(),
			message: "v1".to_string(),
			file: Some("/test".to_owned()),
			static_site_details: StaticSiteDetails {},
		};
		let user_id = get_user_by_username(&mut connection, "testuser")
			.await?
			.unwrap()
			.id;

		// Create static site
		create_resource(
			&mut connection,
			&static_site_id,
			rbac::RESOURCE_TYPES
				.get()
				.unwrap()
				.get(rbac::resource_types::STATIC_SITE)
				.unwrap(),
			&workspace_id,
			&Utc::now(),
		)
		.await?;
		create_static_site(
			&mut connection,
			&static_site_id,
			&static_site.name,
			&workspace_id,
		)
		.await?;
		// Test
		let creation_output =
			get_static_site_by_id(&mut connection, &static_site_id)
				.await?
				.unwrap();
		assert_eq!(static_site.name, creation_output.name);
		assert_eq!(static_site_id, creation_output.id);
		assert_eq!(workspace_id, creation_output.workspace_id);
		assert_eq!(DeploymentStatus::Created, creation_output.status);

		// Create static site upload
		let static_site_upload_id =
			generate_new_resource_id(&mut connection).await?;
		create_resource(
			&mut connection,
			&static_site_upload_id,
			rbac::RESOURCE_TYPES
				.get()
				.unwrap()
				.get(rbac::resource_types::STATIC_SITE_UPLOAD)
				.unwrap(),
			&workspace_id,
			&Utc::now(),
		)
		.await?;
		create_static_site_upload_history(
			&mut connection,
			&static_site_upload_id,
			&static_site_id,
			&static_site.message,
			&user_id,
			&Utc::now(),
		)
		.await?;
		// Test
		let upload_output =
			get_static_site_upload_history(&mut connection, &static_site_id)
				.await?;
		let upload_id_output = get_static_site_upload_history_by_upload_id(
			&mut connection,
			&static_site_id,
			&static_site_upload_id,
		)
		.await?
		.unwrap();
		assert_eq!(static_site_upload_id, upload_output[0].id);
		assert_eq!(static_site.message, upload_output[0].message);
		assert_eq!(user_id, upload_output[0].uploaded_by);
		assert_eq!(static_site_upload_id, upload_id_output.id);
		assert_eq!(static_site.message, upload_id_output.message);
		assert_eq!(user_id, upload_id_output.uploaded_by);

		// Set static site upload as processed
		let processed_time = Utc::now();
		set_static_site_upload_as_processed(
			&mut connection,
			&static_site_id,
			&static_site_upload_id,
			Some(&processed_time),
		)
		.await?;
		// Test
		let processed_output =
			get_static_site_upload_history(&mut connection, &static_site_id)
				.await?;
		assert_eq!(
			processed_time.to_rfc2822(),
			processed_output[0].processed.unwrap().to_rfc2822()
		);

		// Update current live upload
		let no_upload_output =
			get_static_site_by_id(&mut connection, &static_site_id)
				.await?
				.unwrap();
		update_current_live_upload_for_static_site(
			&mut connection,
			&static_site_id,
			&static_site_upload_id,
		)
		.await?;
		// Test
		let current_upload_output =
			get_static_site_by_id(&mut connection, &static_site_id)
				.await?
				.unwrap();
		assert_eq!(None, no_upload_output.current_live_upload);
		assert_eq!(
			static_site_upload_id,
			current_upload_output.current_live_upload.unwrap()
		);

		// Start stop static site
		update_static_site_status(
			&mut connection,
			&static_site_id,
			&DeploymentStatus::Running,
		)
		.await?;
		let running_output =
			get_static_site_by_id(&mut connection, &static_site_id)
				.await?
				.unwrap()
				.status;
		update_static_site_status(
			&mut connection,
			&static_site_id,
			&DeploymentStatus::Errored,
		)
		.await?;
		let errored_output =
			get_static_site_by_id(&mut connection, &static_site_id)
				.await?
				.unwrap()
				.status;
		// Test
		assert_eq!(DeploymentStatus::Running, running_output);
		assert_eq!(DeploymentStatus::Errored, errored_output);

		// Update static site name
		update_static_site_name(&mut connection, &static_site_id, "v2").await?;
		// Test
		let name_change_output =
			get_static_site_by_id(&mut connection, &static_site_id)
				.await?
				.unwrap()
				.name;
		assert_eq!("v2".to_string(), name_change_output);

		// Delete static site
		delete_static_site(&mut connection, &static_site_id, &Utc::now())
			.await?;
		// Test for deletion of static site
		// Get static site by calling get_static_site_for_workspace
		let deleted_output =
			get_static_site_by_id(&mut connection, &static_site_id).await?;
		assert_eq!(None, deleted_output);

		deinit_test(app.config.database.database).await?;
		Ok(())
	}
}
