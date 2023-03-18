use std::time::Duration;

use api_models::models::workspace::{
	domain::DnsRecordValue,
	region::RegionStatus,
};
use eve_rs::AsError;
use kube::config::Kubeconfig;
use reqwest::Client;
use sqlx::Connection;
use tokio::{fs, process::Command};

use crate::{
	db,
	models::{
		digitalocean::ClusterState,
		rabbitmq::{BYOCData, InfraRequestData},
	},
	service,
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
			kube_config,
			tls_cert,
			tls_key,
			request_id,
		} => {
			let Some(region) =
				db::get_region_by_id(connection, &region_id).await? else {
				log::error!(
					"request_id: {} - Unable to find region with ID `{}`",
					request_id,
					&region_id
				);
				return Ok(());
			};

			if region.status != RegionStatus::Creating {
				log::error!(
					concat!(
						"request_id: {} - Status of region {} is {:?}, so",
						" dropping init msg in rabbitmq as it is not ",
						"in `creating` state"
					),
					request_id,
					region_id,
					region.status,
				);
				return Ok(());
			}

			let kubeconfig_path = format!("init-kubeconfig-{region_id}.yaml");
			fs::write(&kubeconfig_path, serde_yaml::to_string(&kube_config)?)
				.await?;

			let tls_cert_path = format!("tls-cert-{region_id}.cert");
			fs::write(&tls_cert_path, &tls_cert).await?;

			let tls_key_path = format!("tls-key-{region_id}.key");
			fs::write(&tls_key_path, &tls_key).await?;

			// safe to return as only customer cluster is initalized here,
			// so workspace_id will be present
			let parent_workspace = region.workspace_id.status(500)?.to_string();

			let output = Command::new("assets/k8s/fresh/k8s_init.sh")
				.args([
					region_id.as_str(),
					&parent_workspace,
					&kubeconfig_path,
					&tls_cert_path,
					&tls_key_path,
				])
				.output()
				.await?;

			let std_out = String::from_utf8_lossy(&output.stdout);
			db::append_messge_log_for_region(connection, &region_id, &std_out)
				.await?;

			if !output.status.success() {
				let std_err = String::from_utf8_lossy(&output.stderr);
				log::debug!(
                    "Error while initializing the cluster {}:\nStatus: {}\nStdout: {}\nStderr: {}",
                    region_id,
                    output.status,
                    std_out,
                    std_err,
                );
				db::append_messge_log_for_region(
					connection, &region_id, &std_err,
				)
				.await?;
				// don't requeue
				return Ok(());
			}

			log::info!("Initialized cluster {region_id} successfully");

			service::send_message_to_infra_queue(
				&InfraRequestData::BYOC(BYOCData::CheckClusterForReadiness {
					region_id: region_id.clone(),
					kube_config,
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
			kube_config,
			request_id,
		} => {
			log::info!(
				concat!(
					"request_id: {} - Checking readiness of external ",
					"load balancer host in cluster {}"
				),
				request_id,
				region_id
			);

			let Some(region) =
				db::get_region_by_id(connection, &region_id).await? else {
				log::error!(
					"request_id: {} - Unable to find region with ID `{}`",
					request_id,
					&region_id
				);
				return Ok(());
			};

			if region.status != RegionStatus::Creating {
				log::error!(
					concat!(
						"request_id: {} - Status of region {} is {:?}, so",
						" dropping init msg in rabbitmq as it is not ",
						"in `creating` state"
					),
					request_id,
					region_id,
					region.status,
				);
				return Ok(());
			}

			let ingress_hostname =
				service::get_patr_ingress_load_balancer_hostname(
					kube_config.clone(),
				)
				.await;

			match ingress_hostname {
				Err(err) => {
					log::info!(
						"Error while getting hostname for ingress - {}",
						err.get_error()
					);
					log::info!("So marking the cluster {region_id} as errored");
					db::set_region_as_errored(connection, &region_id).await?;

					Ok(())
				}
				Ok(None) => {
					// Retry in 1 min
					tokio::time::sleep(Duration::from_secs(60)).await;
					service::send_message_to_infra_queue(
						&InfraRequestData::BYOC(
							BYOCData::CheckClusterForReadiness {
								region_id: region_id.clone(),
								kube_config,
								request_id: request_id.clone(),
							},
						),
						config,
						&request_id,
					)
					.await?;

					Ok(())
				}
				Ok(Some(hostname)) => {
					let mut connection = connection.begin().await?;

					db::set_region_as_active(
						&mut connection,
						&region_id,
						kube_config,
						&hostname,
					)
					.await?;

					db::append_messge_log_for_region(
						&mut connection,
						&region_id,
						concat!(
							"Successfully assigned host for load balancer.\n",
							"Region is now ready for deployments.\n"
						),
					)
					.await?;

					let onpatr_domain = db::get_domain_by_name(
						&mut connection,
						&config.cloudflare.onpatr_domain,
					)
					.await?
					.status(500)?;

					let resource = db::get_resource_by_id(
						&mut connection,
						&onpatr_domain.id,
					)
					.await?
					.status(500)?;

					let dns_record = match hostname {
						url::Host::Domain(domain) => DnsRecordValue::CNAME {
							target: domain,
							proxied: false,
						},
						url::Host::Ipv4(ip_v4) => DnsRecordValue::A {
							target: ip_v4,
							proxied: false,
						},
						url::Host::Ipv6(ip_v6) => DnsRecordValue::AAAA {
							target: ip_v6,
							proxied: false,
						},
					};

					service::create_patr_domain_dns_record(
						&mut connection,
						&resource.owner_id,
						&onpatr_domain.id,
						&format!("*.{}", region_id),
						1, // 1 means 'automatic'
						&dns_record,
						config,
						&request_id,
					)
					.await
					.map_err(|err| {
						log::error!(
							concat!(
								"request_id: {} Error",
								" creating DNS for region {}",
							),
							request_id,
							region_id
						);
						err
					})?;

					connection.commit().await?;

					Ok(())
				}
			}
		}
		BYOCData::GetDigitalOceanKubeconfig {
			api_token,
			cluster_id,
			region_id,
			tls_cert,
			tls_key,
			request_id,
		} => {
			log::trace!(
				"request_id: {} checking for readiness and getting kubeconfig",
				request_id
			);

			let Some(region) =
				db::get_region_by_id(connection, &region_id).await?
			else {
				log::error!(
					"request_id: {} - Unable to find region with ID `{}`",
					request_id,
					&region_id
				);
				return Ok(());
			};

			if region.status != RegionStatus::Creating {
				log::error!(
					concat!(
						"request_id: {} - Status of region {} is {:?}, so",
						" dropping init msg in rabbitmq as it is not ",
						"in `creating` state"
					),
					request_id,
					region_id,
					region.status,
				);
				return Ok(());
			}

			let client = Client::new();
			let cluster_info = client
				.get(format!(
					"https://api.digitalocean.com/v2/kubernetes/clusters/{}",
					cluster_id
				))
				.bearer_auth(api_token.clone())
				.send()
				.await?
				.json::<serde_json::Value>()
				.await
				.map_err(|err| {
					log::error!("Error while parsing cluster info - {err}");
					err
				})
				.ok();

			let cluster_state = cluster_info
				.as_ref()
				.and_then(|cluster_info| cluster_info.get("kubernetes_cluster"))
				.and_then(|cluster_info| cluster_info.get("status"))
				.and_then(|status| status.get("state"))
				.map(|state| {
					serde_json::from_value::<ClusterState>(state.to_owned())
						.unwrap_or_else(|err| {
							log::error!(
								"Error while parsing cluster state - {err}"
							);
							ClusterState::Error
						})
				})
				.unwrap_or(ClusterState::Error);

			match cluster_state {
				ClusterState::Provisioning => {
					log::info!(
						concat!(
							"request_id: {} Cluster is privisioning state.",
							" Trying again after 1min..."
						),
						request_id,
					);
					// Check again in 1 min
					tokio::time::sleep(Duration::from_secs(60)).await;
					service::send_message_to_infra_queue(
						&InfraRequestData::BYOC(
							BYOCData::GetDigitalOceanKubeconfig {
								api_token,
								cluster_id,
								region_id,
								tls_cert,
								tls_key,
								request_id: request_id.clone(),
							},
						),
						config,
						&request_id,
					)
					.await?;

					Ok(())
				}
				ClusterState::Running => {
					// cluster ready
					let do_cluster_url = format!(
						concat!(
							"https://api.digitalocean.com/v2",
							"/kubernetes/clusters/{}/kubeconfig"
						),
						cluster_id
					);
					let response = client
						.get(do_cluster_url)
						.bearer_auth(api_token.clone())
						.send()
						.await?;
					let kube_config = response.text().await?;
					let Ok(kube_config) = Kubeconfig::from_yaml(&kube_config) else {
						db::append_messge_log_for_region(
							connection,
							&region_id,
							concat!(
								"Received invalid kubeconfig from DigitalOcean",
								".\nYou can download the KubeConfig from your ",
								"DigitalOcean dashboard, and add your cluster ",
								"manually to Patr"
							),
						)
						.await?;
						db::set_region_as_errored(connection, &region_id).await?;

						return Ok(());
					};

					// initialize the cluster with patr script
					service::send_message_to_infra_queue(
						&InfraRequestData::BYOC(
							BYOCData::InitKubernetesCluster {
								region_id,
								kube_config,
								tls_cert,
								tls_key,
								request_id: request_id.clone(),
							},
						),
						config,
						&request_id,
					)
					.await?;

					Ok(())
				}
				remaining_state => {
					// unknown error occurred while creating cluster in do, so
					// log and then quit
					log::error!(
						concat!(
							"request_id: {} Cluster state is not expected",
							" := `{:?}`. So marking the cluster as errored"
						),
						request_id,
						remaining_state,
					);

					db::append_messge_log_for_region(
						connection,
						&region_id,
						concat!(
							"Error occurred while initialing the",
							" cluster in DO, try creating a new cluster again"
						),
					)
					.await?;

					db::set_region_as_errored(connection, &region_id).await?;

					Ok(())
				}
			}
		}
		BYOCData::DeleteKubernetesCluster {
			region_id,
			workspace_id,
			kube_config,
			request_id,
		} => {
			log::trace!(
				"request_id: {} uninitializing region with ID: {}",
				request_id,
				region_id
			);

			let kubeconfig_path = format!("uninit-kubeconfig-{region_id}.yaml");

			fs::write(&kubeconfig_path, &serde_yaml::to_string(&kube_config)?)
				.await?;

			let output = Command::new("assets/k8s/fresh/k8s_uninit.sh")
				.args([
					region_id.as_str(),
					workspace_id.as_str(),
					&kubeconfig_path,
				])
				.output()
				.await?;

			let std_out = String::from_utf8_lossy(&output.stdout);
			db::append_messge_log_for_region(connection, &region_id, &std_out)
				.await?;

			if !output.status.success() {
				let std_err = String::from_utf8_lossy(&output.stderr);
				log::debug!(
					concat!(
						"Error while un-initializing the cluster {}:",
						"\nStatus: {}\nStdout: {}\nStderr: {}"
					),
					region_id,
					output.status,
					std_out,
					std_err
				);
				db::append_messge_log_for_region(
					connection, &region_id, &std_err,
				)
				.await?;

				// don't requeue
				return Ok(());
			}

			log::info!("Un-Initialized cluster {region_id} successfully");
			Ok(())
		}
	}
}
