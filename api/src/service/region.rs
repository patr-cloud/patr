use api_models::{
	models::workspace::{
		domain::DnsRecordValue,
		region::{AddRegionToWorkspaceData, InfrastructureCloudProvider},
	},
	utils::Uuid,
};
use chrono::Utc;
use eve_rs::AsError;
use reqwest::Url;

use crate::{
	db,
	error,
	models::rbac::{self, resource_types},
	service,
	utils::{settings::Settings, Error},
	Database,
};

pub async fn add_new_region_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	name: &str,
	data: &AddRegionToWorkspaceData,
	config: &Settings,
	request_id: &Uuid,
) -> Result<Uuid, Error> {
	let region_id = db::generate_new_resource_id(connection).await?;

	db::create_resource(
		connection,
		&region_id,
		&format!("Region: {}", name),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(resource_types::DEPLOYMENT_REGION)
			.unwrap(),
		workspace_id,
		&Utc::now(),
	)
	.await?;

	match data {
		AddRegionToWorkspaceData::Digitalocean {
			cloud_provider: _,
			region: _,
			api_token: _,
		} => todo!(),
		AddRegionToWorkspaceData::KubernetesCluster {
			certificate_authority_data,
			cluster_url,
			auth_username,
			auth_token,
		} => {
			db::add_deployment_region_to_workspace(
				connection,
				&region_id,
				name,
				&InfrastructureCloudProvider::Other,
				workspace_id,
			)
			.await?;

			let url = Url::parse(cluster_url)?;

			let patr_domain = db::get_domain_by_name(connection, "patr.cloud")
				.await?
				.status(500)?;

			let resource = db::get_resource_by_id(connection, &patr_domain.id)
				.await?
				.status(500)?;

			service::create_patr_domain_dns_record(
				connection,
				&resource.owner_id,
				&patr_domain.id,
				region_id.as_str(),
				0,
				&DnsRecordValue::CNAME {
					target: url
						.domain()
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?
						.to_string(),
					proxied: false,
				},
				config,
				request_id,
			)
			.await?;

			service::queue_setup_kubernetes_cluster(
				&region_id,
				cluster_url,
				certificate_authority_data,
				auth_username,
				auth_token,
				config,
				request_id,
			)
			.await?;
		}
	}

	Ok(region_id)
}
