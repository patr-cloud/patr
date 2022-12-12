use std::time::Duration;

use api_models::models::workspace::domain::DnsRecordValue;
use eve_rs::AsError;
use tokio::{fs, process::Command};

use crate::{
	db,
	models::rabbitmq::{BYOCData, InfraRequestData},
	service::{self, KubernetesAuthDetails},
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn process_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	request_data: BYOCData,
	config: &Settings,
) -> Result<(), Error> {
	match request_data {
		BYOCData::InitKubernetesCluster {
			region_id,
			cluster_url,
			certificate_authority_data,
			auth_username,
			auth_token,
			request_id,
		} => {
			let region = if let Some(region) =
				db::get_region_by_id(connection, &region_id).await?
			{
				region
			} else {
				log::error!(
					"request_id: {} - Unable to find region with ID `{}`",
					request_id,
					&region_id
				);
				return Ok(());
			};

			let kubeconfig_content = generate_kubeconfig_from_template(
				&cluster_url,
				&auth_username,
				&auth_token,
				&certificate_authority_data,
			);

			let kubeconfig_path = format!("{region_id}.yml");

			fs::write(&kubeconfig_path, &kubeconfig_content).await?;

			// safe to return as only customer cluster is initalized here,
			// so workspace_id will be present
			let parent_workspace = region
				.workspace_id
				.map(|id| id.as_str().to_owned())
				.status(500)?;

			// todo: get both stdout and stderr in same stream -> use subprocess crate in future
			let output = Command::new("assets/k8s/fresh/k8s_init.sh")
				.args([
					region_id.as_str(),
					&parent_workspace,
					&kubeconfig_path,
				])
				.output()
				.await?;

			db::append_messge_log_for_region(
				connection,
				&region_id,
				std::str::from_utf8(&output.stdout)?,
			)
			.await?;

			if !output.status.success() {
				log::debug!(
                    "Error while initializing the cluster {}:\nStatus: {}\nStderr: {}\nStdout: {}",
                    region_id,
                    output.status,
                    String::from_utf8_lossy(&output.stderr),
                    String::from_utf8_lossy(&output.stdout)
                );
				db::append_messge_log_for_region(
					connection,
					&region_id,
					std::str::from_utf8(&output.stderr)?,
				)
				.await?;
				// don't requeue
				return Ok(());
			}

			log::info!("Initialized cluster {region_id} successfully");

			service::send_message_to_infra_queue(
				&InfraRequestData::BYOC(BYOCData::CheckClusterForReadiness {
					region_id: region_id.clone(),
					cluster_url: cluster_url.to_owned(),
					certificate_authority_data: certificate_authority_data
						.to_string(),
					auth_username: auth_username.to_string(),
					auth_token: auth_token.to_string(),
					request_id: request_id.clone(),
				}),
				config,
				&request_id,
			)
			.await?;

			Ok(())
		}
		BYOCData::CheckClusterForReadiness {
			region_id,
			cluster_url,
			certificate_authority_data,
			auth_username,
			auth_token,
			request_id,
		} => {
			log::info!("Checking readiness of external load balancer ip in cluster {region_id}");

			let ip_addr = service::get_external_ip_addr_for_load_balancer(
				"ingress-nginx",
				"ingress-nginx-controller",
				KubernetesAuthDetails {
					cluster_url: cluster_url.clone(),
					auth_username: auth_username.clone(),
					auth_token: auth_token.clone(),
					certificate_authority_data: certificate_authority_data
						.clone(),
				},
			)
			.await?;

			let ip_addr = match ip_addr {
				Some(ip_addr) => ip_addr,
				None => {
					// if ip is not ready yet, then wait for two mins and then
					// check again
					tokio::time::sleep(Duration::from_secs(2 * 60)).await;
					service::send_message_to_infra_queue(
						&InfraRequestData::BYOC(
							BYOCData::CheckClusterForReadiness {
								region_id: region_id.clone(),
								cluster_url: cluster_url.to_owned(),
								certificate_authority_data:
									certificate_authority_data.to_string(),
								auth_username: auth_username.to_string(),
								auth_token: auth_token.to_string(),
								request_id: request_id.clone(),
							},
						),
						config,
						&request_id,
					)
					.await?;
					return Ok(());
				}
			};

			let region = db::get_region_by_id(connection, &region_id)
				.await?
				.status(500)?;

			service::create_external_service_for_region(
				region.workspace_id.as_ref().status(500)?.as_str(),
				&region_id,
				&ip_addr,
				service::get_kubernetes_config_for_default_region(config)
					.auth_details,
			)
			.await?;

			let patr_domain = db::get_domain_by_name(
				connection,
				"patr.cloud",
			)
			.await?
			.status(500)?;

			let resource = db::get_resource_by_id(
				connection,
				&patr_domain.id,
			)
			.await?
			.status(500)?;


			let dns_record = match ip_addr {
				std::net::IpAddr::V4(ip_v4) => DnsRecordValue::A { target: ip_v4, proxied: false },
				std::net::IpAddr::V6(ip_v6) => DnsRecordValue::AAAA { target: ip_v6, proxied: false },
			};

			service::create_patr_domain_dns_record(
				connection,
				&resource.owner_id,
				&patr_domain.id,
				region_id.as_str(),
				0,
				&dns_record,
				config,
				&request_id,
			)
			.await?;

			db::mark_deployment_region_as_ready(
				connection,
				&region_id,
				&cluster_url,
				&auth_username,
				&auth_token,
				&certificate_authority_data,
				&ip_addr,
			)
			.await?;

			db::append_messge_log_for_region(
				connection,
				&region_id,
				"Successfully assigned IP Addr for load balancer.\nRegion is now ready for deployments"
			).await?;

			service::update_cloudflare_kv_for_region(&region_id, &ip_addr.to_string(), config).await?;

			Ok(())
		}
		BYOCData::CreateDigitaloceanCluster {
			region_id: _,
			digitalocean_region: _,
			access_token: _,
			request_id: _,
		} => Err(
			Error::empty().body("Currently creating cluster through digital ocean cluster is not supported")
		),
	}
}

fn generate_kubeconfig_from_template(
	cluster_url: &str,
	auth_username: &str,
	auth_token: &str,
	certificate_authority_data: &str,
) -> String {
	format!(
		r#"apiVersion: v1
kind: Config
clusters:
  - name: kubernetesCluster
    cluster:
      certificate-authority-data: {certificate_authority_data}
      server: {cluster_url}
users:
  - name: {auth_username}
    user:
      token: {auth_token}
contexts:
  - name: kubernetesContext
    context:
      cluster: kubernetesCluster
      user: {auth_username}
current-context: kubernetesContext
preferences: {{}}"#
	)
}
