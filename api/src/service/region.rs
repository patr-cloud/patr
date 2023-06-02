use api_models::{models::workspace::region::RegionStatus, utils::Uuid};
use eve_rs::{AsError, Error as _};
use kube::config::Kubeconfig;
use reqwest::Client;
use serde_json::json;
use sqlx::types::Json;

use crate::{
	db,
	error,
	models::digitalocean::{K8NodePool, K8sConfig, K8sCreateCluster},
	utils::Error,
	Database,
};

pub async fn get_kubernetes_config_for_region(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
) -> Result<(Kubeconfig, Uuid), Error> {
	let region = db::get_region_by_id(connection, region_id)
		.await?
		.status(500)?;

	match (
		region.status,
		region.workspace_id,
		region.config_file,
		region.ingress_hostname,
	) {
		(RegionStatus::Active, _, Some(Json(config)), Some(_)) => {
			// If the region is active, regardless of the workspace, use the
			// config
			Ok((config, region.id))
		}
		(RegionStatus::ComingSoon, None, ..) => {
			// If the region is default, and is coming soon, get the first
			// default valid region
			let deployed_region = db::get_all_default_regions(connection)
				.await?
				.into_iter()
				.find(|region| region.status == RegionStatus::Active)
				.status(500)?;
			Ok((
				deployed_region.config_file.status(500)?.0,
				deployed_region.id,
			))
		}
		_ => Err(Error::empty()
			.status(400)
			.body(error!(REGION_NOT_READY_YET).to_string())),
	}
}

pub async fn create_do_k8s_cluster(
	region: &str,
	api_token: &str,
	cluster_name: &str,
	min_nodes: u16,
	max_nodes: u16,
	auto_scale: bool,
	node_name: &str,
	node_size_slug: &str,
	request_id: &Uuid,
) -> Result<Uuid, Error> {
	log::trace!(
		"request_id: {} creating digital ocean k8s cluster using api call",
		request_id
	);
	let client = Client::new();

	let response = client
		.post("https://api.digitalocean.com/v2/kubernetes/clusters")
		.bearer_auth(api_token)
		.json(&K8sConfig {
			region: region.to_string(),
			version: "latest".to_string(),
			name: cluster_name.to_string(),
			node_pools: vec![K8NodePool {
				name: node_name.to_string(),
				size: node_size_slug.to_string(),
				auto_scale,
				min_nodes,
				max_nodes,
			}],
		})
		.send()
		.await?;

	match response.error_for_status_ref() {
		Ok(_) => {
			let k8s_cluster_id = response
				.json::<K8sCreateCluster>()
				.await?
				.kubernetes_cluster
				.id;

			Ok(k8s_cluster_id)
		}
		Err(_) => {
			let err_status_code = response.status().as_u16();
			let err_response = response.json::<serde_json::Value>().await?;
			let do_error_msg = err_response
				.get("message")
				.and_then(|value| value.as_str())
				.unwrap_or_default();
			Err(Error::empty().status(err_status_code).body(
				json!({
					"success": false,
					"error": "digitaloceanError",
					"message": do_error_msg,
				})
				.to_string(),
			))
		}
	}
}

pub async fn is_deployed_on_patr_cluster(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
) -> Result<bool, Error> {
	let region = db::get_region_by_id(connection, region_id)
		.await?
		.status(500)?;
	Ok(region.workspace_id.is_none())
}
