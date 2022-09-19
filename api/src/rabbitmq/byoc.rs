use std::time::Duration;

use api_models::utils::Uuid;
use tokio::{fs, process::Command};

use crate::{
	db,
	models::rabbitmq::BYOCData,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn process_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	request_data: BYOCData,
	_config: &Settings,
) -> Result<(), Error> {
	match request_data {
		BYOCData::SetupKubernetesCluster {
			region_id,
			cluster_url,
			certificate_authority_data,
			auth_username,
			auth_token,
			request_id,
		} => {
			let _region = if let Some(region) =
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

			initialize_k8s_cluster(
				connection,
				region_id,
				&cluster_url,
				&auth_username,
				&auth_token,
				&certificate_authority_data,
			)
			.await?;

			Ok(())
		}
		BYOCData::CreateDigitaloceanCluster {
			region_id: _,
			digitalocean_region: _,
			access_token: _,
			request_id: _,
		} => Ok(()),
	}
}

async fn initialize_k8s_cluster(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: Uuid,
	cluster_url: &str,
	auth_user: &str,
	auth_token: &str,
	certficate_authority_data: &str,
) -> Result<(), Error> {
	let kubeconfig_content = generate_kubeconfig_from_template(
		cluster_url,
		auth_user,
		auth_token,
		certficate_authority_data,
	);

	let kubeconfig_path = format!("{region_id}.yml");

	// todo: use temp file and clean up resources
	fs::write(&kubeconfig_path, &kubeconfig_content).await?;

	let output = Command::new("k8s/fresh/k8s_init.sh")
		.args(&[region_id.as_str(), &kubeconfig_path])
		.output()
		.await?;

	if !output.status.success() {
		log::info!("Error while initializing cluster {region_id}\n{output:?}");
		tokio::time::sleep(Duration::from_secs(5)).await;
		log::info!("Retry initializing the cluster {region_id}");
	}

	db::append_messge_log_for_region(
		connection,
		&region_id,
		std::str::from_utf8(&output.stdout)?,
	).await?;

	db::mark_deployment_region_as_ready(
		connection,
		&region_id,
		cluster_url,
		auth_user,
		auth_token,
		certficate_authority_data,
	)
	.await?;

	log::info!("Initialized cluster {region_id} successfully");

	Ok(())
}

fn generate_kubeconfig_from_template(
	cluster_url: &str,
	auth_user: &str,
	auth_token: &str,
	certificate_authority_data: &str,
) -> String {
	format!(
		r#"
apiVersion: v1
kind: Config
clusters:
  - name: kubernetesCluster
    cluster:
      certificate-authority-data: {certificate_authority_data}
      server: {cluster_url}
users:
  - name: {auth_user}
    user:
      token: {auth_token}
contexts:
  - name: kubernetesCluster
    context:
      cluster: kubernetesCluster
      user: {auth_user}
current-context: kubernetesCluster
preferences: {{}}
		"#
	)
}
