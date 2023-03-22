#[cfg(test)]
mod tests {
	use api_models::{
		models::workspace::{
			infrastructure::deployment::{DeploymentStatus, ExposedPortType},
			region::RegionStatus,
		},
		utils::Uuid,
	};
	use chrono::Utc;

	use crate::{
		db::*,
		models::rbac,
		test::{deinit_test, user_constaints},
		utils::Error,
	};

	#[tokio::test]
	async fn deployment_external_registry_test() -> Result<(), Error> {
		let app = user_constaints().await?;
		let mut connection = app.database.acquire().await?;
		let workspace_id =
			get_workspace_by_name(&mut connection, "test-workspace")
				.await?
				.unwrap()
				.id;

		let region_id = get_all_default_regions(&mut connection)
			.await?
			.into_iter()
			.find(|region| region.status == RegionStatus::Active)
			.unwrap()
			.id;
		let machine =
			&get_all_deployment_machine_types(&mut connection).await?[0];

		let deployment = Deployment {
			id: Uuid::new_v4(),
			name: "test".to_owned(),
			registry: "docker.io".to_owned(),
			repository_id: None,
			image_name: Some("test_image".to_owned()),
			image_tag: "test_image".to_owned(),
			status: DeploymentStatus::Created,
			workspace_id: workspace_id.clone(),
			region: region_id,
			min_horizontal_scale: 1,
			max_horizontal_scale: 2,
			machine_type: machine.id.clone(),
			deploy_on_push: true,
			startup_probe_path: Some("/data".to_owned()),
			startup_probe_port: Some(2020),
			liveness_probe_path: Some("/data".to_owned()),
			liveness_probe_port: Some(2020),
			current_live_digest: Some("test_digest".to_owned()),
		};
		create_resource(
			&mut connection,
			&deployment.id,
			rbac::RESOURCE_TYPES
				.get()
				.unwrap()
				.get(rbac::resource_types::DEPLOYMENT)
				.unwrap(),
			&workspace_id,
			&Utc::now(),
		)
		.await?;

		// Creating a deployment
		create_deployment_with_external_registry(
			&mut connection,
			&deployment.id,
			&deployment.name,
			&deployment.registry,
			&deployment.image_name.clone().unwrap().as_str(),
			&deployment.image_tag,
			&deployment.workspace_id,
			&deployment.region,
			&deployment.machine_type,
			deployment.deploy_on_push,
			deployment.min_horizontal_scale as u16,
			deployment.max_horizontal_scale as u16,
			None,
			None,
		)
		.await?;

		let output = get_deployments_for_workspace(
			&mut connection,
			&deployment.workspace_id,
		)
		.await?;

		let id_output = get_deployment_by_id(&mut connection, &deployment.id)
			.await?
			.unwrap();

		// Test cases for deployment creation
		assert_eq!(deployment.id, output[0].id);
		assert_eq!(deployment.deploy_on_push, id_output.deploy_on_push);
		assert_ne!(deployment.min_horizontal_scale, 4);

		// Update deployment status
		update_deployment_status(
			&mut connection,
			&deployment.id,
			&DeploymentStatus::Running,
		)
		.await?;
		let output_for_status_update = get_deployments_for_workspace(
			&mut connection,
			&deployment.workspace_id,
		)
		.await?;

		//Test for status update
		assert_eq!(
			DeploymentStatus::Running,
			output_for_status_update[0].status
		);

		// Deploymet port test
		let expected_port = 2020;
		let expected_port_type = ExposedPortType::Http;

		add_exposed_port_for_deployment(
			&mut connection,
			&deployment.id,
			expected_port,
			&expected_port_type,
		)
		.await?;
		let port_output =
			get_exposed_ports_for_deployment(&mut connection, &deployment.id)
				.await?;

		// Test after adding port
		assert_eq!(expected_port, port_output[0].0);
		assert_eq!(expected_port_type, port_output[0].1);

		remove_all_exposed_ports_for_deployment(
			&mut connection,
			&deployment.id,
		)
		.await?;
		let delete_output =
			get_exposed_ports_for_deployment(&mut connection, &deployment.id)
				.await?;

		// Test after removing port
		assert_eq!(expected_port, delete_output[0].0);
		assert_eq!(expected_port_type, delete_output[0].1);

		// Volume
		let volume_id = Uuid::new_v4();
		let volume_name = "test";
		let volume_size = 16;
		let volume_path = "/root";

		add_volume_for_deployment(
			&mut connection,
			&deployment.id,
			&volume_id,
			&volume_name,
			volume_size,
			&volume_path,
		)
		.await?;
		let vol_output =
			get_all_deployment_volumes(&mut connection, &deployment.id).await?;

		//Test for volume addition
		assert_eq!(volume_id, vol_output[0].volume_id);
		assert_eq!(volume_name, vol_output[0].name);
		assert_eq!(volume_size, vol_output[0].size);
		assert_eq!(volume_path, vol_output[0].path);

		update_volume_for_deployment(
			&mut connection,
			&deployment.id,
			18,
			volume_name,
		)
		.await?;
		let update_output =
			get_all_deployment_volumes(&mut connection, &deployment.id).await?;

		//Test for volume updation
		assert_eq!(volume_id, update_output[0].volume_id);
		assert_eq!(volume_name, update_output[0].name);
		assert_eq!(18, update_output[0].size);
		assert_eq!(volume_path, update_output[0].path);

		delete_volume(&mut connection, &volume_id, &Utc::now()).await?;
		let deleted_output =
			get_all_deployment_volumes(&mut connection, &deployment.id).await?;

		//Test for volume deletion
		assert_ne!(volume_id, deleted_output[0].volume_id);

		// Env variables
		let expected_env_output = DeploymentEnvironmentVariable {
			deployment_id: deployment.id.clone(),
			name: "ENV_TEST".to_owned(), //key
			value: Some("value".to_owned()),
			secret_id: Some(Uuid::new_v4()),
		};
		add_environment_variable_for_deployment(
			&mut connection,
			&deployment.id,
			&expected_env_output.name.as_str(),
			Some(expected_env_output.value.as_deref().unwrap_or("value")),
			expected_env_output.secret_id.as_ref(),
		)
		.await?;
		let env_output = get_environment_variables_for_deployment(
			&mut connection,
			&deployment.id,
		)
		.await?;

		// Test after adding env variable
		assert_eq!(expected_env_output.value, env_output[0].value);
		assert_eq!(deployment.id, env_output[0].deployment_id);
		assert_eq!(expected_env_output.secret_id, env_output[0].secret_id);

		// Removing env variables
		remove_all_environment_variables_for_deployment(
			&mut connection,
			&deployment.id,
		)
		.await?;
		let deleted_env_output = get_environment_variables_for_deployment(
			&mut connection,
			&deployment.id,
		)
		.await?;

		// Test after removing env variables
		assert_ne!(deployment.id, deleted_env_output[0].deployment_id);
		assert_ne!(expected_env_output.value, deleted_env_output[0].value);

		// Deleting deployment
		delete_deployment(&mut connection, &deployment.id, &Utc::now()).await?;
		let delete_output = get_deployment_by_id_including_deleted(
			&mut connection,
			&deployment.id,
		)
		.await?
		.unwrap();

		// Test for deleted deployments
		assert_eq!(deployment.id, delete_output.id);

		deinit_test(app.config.database.database).await?;
		Ok(())
	}

	#[tokio::test]
	async fn deployment_internal_registry_test() -> Result<(), Error> {
		let app = user_constaints().await?;
		let mut connection = app.database.acquire().await?;
		let workspace_id =
			get_workspace_by_name(&mut connection, "test-workspace")
				.await?
				.unwrap()
				.id;
		let region_id = get_all_default_regions(&mut connection)
			.await?
			.into_iter()
			.find(|region| region.status == RegionStatus::Active)
			.unwrap()
			.id;
		let machine =
			&get_all_deployment_machine_types(&mut connection).await?[0];
		let expected_output = Deployment {
			id: Uuid::new_v4(),
			name: "test".to_owned(),
			registry: "registry.patr.cloud".to_owned(),
			repository_id: Some(Uuid::new_v4()),
			image_name: Some("nginx".to_owned()),
			image_tag: "latest".to_owned(),
			status: DeploymentStatus::Created,
			workspace_id: workspace_id.clone(),
			region: region_id.clone(),
			min_horizontal_scale: 1,
			max_horizontal_scale: 2,
			machine_type: machine.id.clone(),
			deploy_on_push: true,
			startup_probe_path: Some("/data".to_owned()),
			startup_probe_port: Some(2020),
			liveness_probe_path: Some("/data".to_owned()),
			liveness_probe_port: Some(2020),
			current_live_digest: Some("test_digest".to_owned()),
		};
		create_resource(
			&mut connection,
			&expected_output.id,
			rbac::RESOURCE_TYPES
				.get()
				.unwrap()
				.get(rbac::resource_types::DEPLOYMENT)
				.unwrap(),
			&workspace_id,
			&Utc::now(),
		)
		.await?;
		create_docker_repository(
			&mut connection,
			&expected_output.id,
			&expected_output.image_name.clone().unwrap(),
			&workspace_id,
		)
		.await?;
		create_deployment_with_internal_registry(
			&mut connection,
			&expected_output.id,
			&expected_output.name,
			&expected_output.id,
			&expected_output.image_tag.clone().as_str(),
			&workspace_id,
			&region_id.clone(),
			&expected_output.machine_type,
			expected_output.deploy_on_push,
			expected_output.min_horizontal_scale as u16,
			expected_output.max_horizontal_scale as u16,
			None,
			None,
		)
		.await?;

		let output = get_deployments_for_workspace(
			&mut connection,
			&expected_output.workspace_id,
		)
		.await?;
		let image_output = get_deployments_by_image_name_and_tag_for_workspace(
			&mut connection,
			&expected_output.image_name.clone().unwrap(),
			&expected_output.image_tag,
			&expected_output.workspace_id,
		)
		.await?;
		let workspace_output = get_deployments_for_workspace(
			&mut connection,
			&expected_output.workspace_id,
		)
		.await?;

		// Test cases
		assert_eq!(
			expected_output.deploy_on_push,
			image_output[0].deploy_on_push
		);
		assert_eq!(expected_output.id, output[0].id);
		assert_eq!(
			expected_output.image_tag,
			image_output[0].image_tag.clone()
		);
		assert_eq!(expected_output.id, image_output[0].id);
		assert_eq!(expected_output.id, workspace_output[0].id);

		deinit_test(app.config.database.database).await?;
		Ok(())
	}
}
