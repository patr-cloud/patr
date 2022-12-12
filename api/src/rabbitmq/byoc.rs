use std::time::Duration;

use eve_rs::AsError;
use kube::config::Kubeconfig;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::{fs, process::Command};

use crate::{
	db,
	models::rabbitmq::{BYOCData, InfraRequestData},
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

			let kubeconfig_path = format!("{region_id}.yml");

			fs::write(&kubeconfig_path, serde_yaml::to_string(&kube_config)?)
				.await?;

			// safe to return as only customer cluster is initalized here,
			// so workspace_id will be present
			let parent_workspace = region
				.workspace_id
				.map(|id| id.as_str().to_owned())
				.status(500)?;

			// todo: get both stdout and stderr in same stream -> use subprocess
			// crate in future
			let output = Command::new("assets/k8s/fresh/k8s_init.sh")
				.args([region_id.as_str(), &parent_workspace, &kubeconfig_path])
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
			log::info!("Checking readiness of external load balancer ip in cluster {region_id}");

			let ingress_hostname = service::get_load_balancer_hostname(
				"ingress-nginx",
				"ingress-nginx-controller",
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
					db::mark_deployment_region_as_errored(
						connection, &region_id,
					)
					.await?;

					Ok(())
				}
				Ok(None) => {
					tokio::time::sleep(Duration::from_secs(2 * 60)).await;
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
					db::mark_deployment_region_as_active(
						connection,
						&region_id,
						kube_config,
						&hostname,
					)
					.await?;

					db::append_messge_log_for_region(
						connection,
						&region_id,
						concat!(
							"Successfully assigned host for load balancer.\n",
							"Region is now ready for deployments.\n"
						),
					)
					.await?;

					Ok(())
				}
			}
		}
		BYOCData::GetDigitalOceanKubeconfig {
			api_token,
			cluster_id,
			region_id,
			request_id,
		} => {
			let client = Client::new();
			log::trace!(
				"request_id: {} checking for readiness and getting kubeconfig",
				request_id
			);

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

			#[derive(Debug, Serialize, Deserialize)]
			#[serde(rename_all = "lowercase")]
			enum ClusterState {
				Running,
				Provisioning,
				Degraded,
				Error,
				Deleted,
				Upgrading,
				Deleting,
			}

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
						"request_id: {} Cluster is privisioning state. Trying again after 2mins...",
						request_id,
					);
					tokio::time::sleep(Duration::from_secs(2 * 60)).await;
					service::send_message_to_infra_queue(
						&InfraRequestData::BYOC(
							BYOCData::GetDigitalOceanKubeconfig {
								api_token,
								cluster_id,
								region_id,
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
						"https://api.digitalocean.com/v2/kubernetes/clusters/{}/kubeconfig",
						cluster_id
					);
					let response = client
						.get(do_cluster_url)
						.bearer_auth(api_token.clone())
						.send()
						.await?;
					let kube_config = response.text().await?;
					let kube_config = Kubeconfig::from_yaml(&kube_config)?;

					// initialize the cluster with patr script
					service::send_message_to_infra_queue(
						&InfraRequestData::BYOC(
							BYOCData::InitKubernetesCluster {
								region_id,
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
				remaining_state => {
					// unknown error occurred while creating cluster in do, so
					// log and then quit
					log::error!(
						"request_id: {} Cluster state is not expected := `{:?}`. So marking the cluster as errored",
						request_id,
						remaining_state,
					);

					db::append_messge_log_for_region(
						connection,
						&region_id,
						"Error occurred while initialing the cluster in DO, try creating a new cluster again",
					)
					.await?;

					db::mark_deployment_region_as_errored(
						connection, &region_id,
					)
					.await?;

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

			let kubeconfig_path = format!("{region_id}.yml");

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

			db::append_messge_log_for_region(
				connection,
				&region_id,
				std::str::from_utf8(&output.stdout)?,
			)
			.await?;

			if !output.status.success() {
				log::debug!(
                    "Error while un-initializing the cluster {}:\nStatus: {}\nStderr: {}\nStdout: {}",
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

			service::update_cloudflare_kv_for_region(&region_id, &ip_addr.to_string(), config).await?;

				return Ok(());
			}

			log::info!("Un-Initialized cluster {region_id} successfully");
			Ok(())
		}
	}
}
