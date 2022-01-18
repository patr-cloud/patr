use api_models::{
	models::workspace::infrastructure::managed_urls::{
		ManagedUrl,
		ManagedUrlType,
	},
	utils::Uuid,
};
use eve_rs::AsError;

use super::kubernetes;
use crate::{
	db,
	error,
	models::{db_mapping::ManagedUrlType as DbManagedUrlType, rbac},
	utils::{get_current_time_millis, settings::Settings, Error},
	Database,
};

pub async fn create_new_managed_url_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	sub_domain: &str,
	domain_id: &Uuid,
	path: &str,
	url_type: &ManagedUrlType,
	config: &Settings,
) -> Result<Uuid, Error> {
	let managed_url_id = db::generate_new_resource_id(connection).await?;

	let domain = db::get_workspace_domain_by_id(connection, domain_id)
		.await?
		.status(500)?;

	db::create_resource(
		connection,
		&managed_url_id,
		&format!("Managed URL: {}", managed_url_id),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::MANAGED_URL)
			.unwrap(),
		workspace_id,
		get_current_time_millis(),
	)
	.await?;

	match url_type {
		ManagedUrlType::ProxyDeployment {
			deployment_id,
			port,
		} => {
			db::create_new_managed_url_in_workspace(
				connection,
				&managed_url_id,
				sub_domain,
				domain_id,
				path,
				&DbManagedUrlType::ProxyToDeployment,
				Some(deployment_id),
				Some(*port),
				None,
				None,
				workspace_id,
			)
			.await?;
		}
		ManagedUrlType::ProxyStaticSite { static_site_id } => {
			db::create_new_managed_url_in_workspace(
				connection,
				&managed_url_id,
				sub_domain,
				domain_id,
				path,
				&DbManagedUrlType::ProxyToStaticSite,
				None,
				None,
				Some(static_site_id),
				None,
				workspace_id,
			)
			.await?;
		}
		ManagedUrlType::ProxyUrl { url } => {
			db::create_new_managed_url_in_workspace(
				connection,
				&managed_url_id,
				sub_domain,
				domain_id,
				path,
				&DbManagedUrlType::ProxyUrl,
				None,
				None,
				None,
				Some(url),
				workspace_id,
			)
			.await?;
		}
		ManagedUrlType::Redirect { url } => {
			db::create_new_managed_url_in_workspace(
				connection,
				&managed_url_id,
				sub_domain,
				domain_id,
				path,
				&DbManagedUrlType::Redirect,
				None,
				None,
				None,
				Some(url),
				workspace_id,
			)
			.await?;
		}
	}

	kubernetes::update_kubernetes_managed_url(
		workspace_id,
		&ManagedUrl {
			id: managed_url_id.clone(),
			sub_domain: sub_domain.to_string(),
			domain_id: domain_id.clone(),
			path: path.to_string(),
			url_type: url_type.clone(),
		},
		config,
		&Uuid::new_v4(),
	)
	.await?;

	if domain.is_ns_external() {
		kubernetes::create_certificates(
			workspace_id,
			&format!("certificate-{}", managed_url_id),
			&format!("tls-{}", managed_url_id),
			vec![format!("{}.{}", sub_domain, domain.name)],
			config,
		)
		.await?;
	}

	Ok(managed_url_id)
}

pub async fn update_managed_url(
	connection: &mut <Database as sqlx::Database>::Connection,
	managed_url_id: &Uuid,
	path: &str,
	url_type: &ManagedUrlType,
	config: &Settings,
) -> Result<(), Error> {
	let managed_url = db::get_managed_url_by_id(connection, managed_url_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	match url_type {
		ManagedUrlType::ProxyDeployment {
			deployment_id,
			port,
		} => {
			db::update_managed_url(
				connection,
				managed_url_id,
				path,
				&DbManagedUrlType::ProxyToDeployment,
				Some(deployment_id),
				Some(*port),
				None,
				None,
			)
			.await?;
		}
		ManagedUrlType::ProxyStaticSite { static_site_id } => {
			db::update_managed_url(
				connection,
				managed_url_id,
				path,
				&DbManagedUrlType::ProxyToStaticSite,
				None,
				None,
				Some(static_site_id),
				None,
			)
			.await?;
		}
		ManagedUrlType::ProxyUrl { url } => {
			db::update_managed_url(
				connection,
				managed_url_id,
				path,
				&DbManagedUrlType::ProxyUrl,
				None,
				None,
				None,
				Some(url),
			)
			.await?;
		}
		ManagedUrlType::Redirect { url } => {
			db::update_managed_url(
				connection,
				managed_url_id,
				path,
				&DbManagedUrlType::Redirect,
				None,
				None,
				None,
				Some(url),
			)
			.await?;
		}
	}

	kubernetes::update_kubernetes_managed_url(
		&managed_url.workspace_id,
		&ManagedUrl {
			id: managed_url.id,
			sub_domain: managed_url.sub_domain,
			domain_id: managed_url.domain_id,
			path: managed_url.path,
			url_type: match managed_url.url_type {
				DbManagedUrlType::ProxyToDeployment => {
					ManagedUrlType::ProxyDeployment {
						deployment_id: managed_url.deployment_id.status(500)?,
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
				DbManagedUrlType::ProxyUrl => ManagedUrlType::ProxyUrl {
					url: managed_url.url.status(500)?,
				},
				DbManagedUrlType::Redirect => ManagedUrlType::Redirect {
					url: managed_url.url.status(500)?,
				},
			},
		},
		config,
		&Uuid::new_v4(),
	)
	.await?;

	Ok(())
}

pub async fn delete_managed_url(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	managed_url_id: &Uuid,
	config: &Settings,
) -> Result<(), Error> {
	db::delete_managed_url(connection, managed_url_id).await?;

	kubernetes::delete_kubernetes_managed_url(
		workspace_id,
		managed_url_id,
		config,
		&Uuid::new_v4(),
	)
	.await?;

	Ok(())
}
