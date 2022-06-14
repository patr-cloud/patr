use api_models::{
	models::workspace::infrastructure::{
		deployment::DeploymentStatus,
		static_site::StaticSiteDetails,
	},
	utils::Uuid,
};

use crate::{
	db,
	models::rabbitmq::StaticSiteRequestData,
	service,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn process_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	request_data: StaticSiteRequestData,
	config: &Settings,
) -> Result<(), Error> {
	match request_data {
		StaticSiteRequestData::Create {
			workspace_id,
			static_site_id,
			upload_id,
			file,
			static_site_details,
			request_id,
		} => {
			log::trace!("request_id: {} - Received create request", request_id);

			db::update_static_site_status(
				connection,
				&static_site_id,
				&DeploymentStatus::Deploying,
			)
			.await?;

			update_static_site_and_db_status(
				connection,
				&workspace_id,
				&static_site_id,
				&upload_id,
				Some(&file),
				&static_site_details,
				config,
				&request_id,
			)
			.await
		}
		StaticSiteRequestData::Start {
			workspace_id,
			static_site_id,
			upload_id,
			static_site_details,
			request_id,
		} => {
			log::trace!(
				"request_id: {} - Received a start request",
				request_id
			);
			update_static_site_and_db_status(
				connection,
				&workspace_id,
				&static_site_id,
				&upload_id,
				None,
				&static_site_details,
				config,
				&request_id,
			)
			.await
		}
		StaticSiteRequestData::Stop {
			workspace_id,
			static_site_id,
			request_id,
		} => {
			log::trace!(
				"request_id: {} - Received a delete static site request",
				request_id
			);
			service::delete_kubernetes_static_site(
				&workspace_id,
				&static_site_id,
				config,
				&request_id,
			)
			.await
		}
		StaticSiteRequestData::UploadSite {
			workspace_id: _,
			static_site_id,
			upload_id,
			file,
			request_id,
		} => {
			log::trace!(
				"request_id: {} - Received a upload static site request",
				request_id
			);
			service::upload_static_site_files_to_s3(
				connection,
				&file,
				&static_site_id,
				&upload_id,
				config,
				&request_id,
			)
			.await
		}
		StaticSiteRequestData::Delete {
			workspace_id,
			static_site_id,
			request_id,
		} => {
			log::trace!(
				"request_id: {} - Received a delete static site request",
				request_id
			);
			service::delete_kubernetes_static_site(
				&workspace_id,
				&static_site_id,
				config,
				&request_id,
			)
			.await
		}
		StaticSiteRequestData::Revert {
			workspace_id,
			static_site_id,
			upload_id,
			static_site_details,
			request_id,
		} => {
			log::trace!(
				"request_id: {} - Received a revert request",
				request_id
			);
			update_static_site_and_db_status(
				connection,
				&workspace_id,
				&static_site_id,
				&upload_id,
				None,
				&static_site_details,
				config,
				&request_id,
			)
			.await
		}
	}
}

async fn update_static_site_and_db_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	static_site_id: &Uuid,
	upload_id: &Uuid,
	file: Option<&str>,
	_running_details: &StaticSiteDetails,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - Updating kubernetes static site",
		request_id
	);
	let result = service::update_kubernetes_static_site(
		workspace_id,
		static_site_id,
		upload_id,
		&StaticSiteDetails {},
		config,
		request_id,
	)
	.await;

	if let Some(file) = file {
		log::trace!(
			"request_id: {} - Uploading static site files to S3",
			request_id
		);
		service::upload_static_site_files_to_s3(
			connection,
			file,
			static_site_id,
			upload_id,
			config,
			request_id,
		)
		.await?;
	}

	if let Err(err) = result {
		log::error!(
			"request_id: {} - Error occured while deploying site `{}`: {}",
			request_id,
			static_site_id,
			err.get_error()
		);
		// TODO log in audit log that there was an error while deploying
		db::update_static_site_status(
			connection,
			static_site_id,
			&DeploymentStatus::Errored,
		)
		.await?;

		Err(err)
	} else {
		db::update_static_site_status(
			connection,
			static_site_id,
			&DeploymentStatus::Running,
		)
		.await?;

		Ok(())
	}
}
