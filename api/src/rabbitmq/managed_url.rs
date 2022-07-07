use api_models::models::workspace::infrastructure::managed_urls::{
	ManagedUrl,
	ManagedUrlType,
};
use eve_rs::AsError;

use crate::{
	db::{self, ManagedUrlType as DbManagedUrlType},
	models::rabbitmq::ManagedUrlData,
	service,
	utils::{settings::Settings, Error},
	Database,
};

pub async fn process_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	request_data: ManagedUrlData,
	config: &Settings,
) -> Result<(), Error> {
	match request_data {
		ManagedUrlData::Create {
			managed_url_id,
			workspace_id,
			request_id,
		} => {
			log::trace!(
				"request_id: {} - Updating managed url on Kubernetes.",
				request_id
			);

			let managed_url = if let Some(managed_url) =
				db::get_managed_url_by_id(connection, &managed_url_id).await?
			{
				managed_url
			} else {
				return Ok(());
			};

			service::update_kubernetes_managed_url(
				&workspace_id,
				&ManagedUrl {
					id: managed_url.id,
					sub_domain: managed_url.sub_domain,
					domain_id: managed_url.domain_id,
					path: managed_url.path,
					url_type: match managed_url.url_type {
						DbManagedUrlType::ProxyToDeployment => {
							ManagedUrlType::ProxyDeployment {
								deployment_id: managed_url
									.deployment_id
									.status(500)?,
								port: managed_url.port.status(500)? as u16,
							}
						}
						DbManagedUrlType::ProxyToStaticSite => {
							ManagedUrlType::ProxyStaticSite {
								static_site_id: managed_url
									.static_site_id
									.status(500)?,
							}
						}
						DbManagedUrlType::ProxyUrl => {
							ManagedUrlType::ProxyUrl {
								url: managed_url.url.status(500)?,
							}
						}
						DbManagedUrlType::Redirect => {
							ManagedUrlType::Redirect {
								url: managed_url.url.status(500)?,
							}
						}
					},
				},
				config,
				&request_id,
			)
			.await
		}
	}
}
