use api_macros::closure_as_pinned_box;
use api_models::{
	models::prelude::*,
	utils::{DateTime, DecodedRequest, Paginated, Uuid},
};
use axum::{extract::State, Extension};
use chrono::Utc;
use sqlx::types::Json;

use crate::{
	app::App,
	db,
	error,
	models::{
		rbac::{self, permissions},
		UserAuthenticationData,
	},
	prelude::*,
	routes::ClientIp,
	service,
	utils::{constants::request_keys, Error},
};

pub fn create_sub_app(app: &App) -> Router<App> {
	let mut app = create_axum_router(app);

	// List all regions
	app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::region::LIST,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(list_regions)),
		],
	);

	// Get region
	app.get(
		"/:regionId",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::region::INFO,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let region_id =
						context.get_param(request_keys::REGION_ID).unwrap();
					let region_id = Uuid::parse_str(region_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &region_id)
							.await?
							.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(get_region)),
		],
	);

	// Check region status
	app.post(
		"/:regionId/checkStatus",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::region::CHECK_STATUS,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let region_id =
						context.get_param(request_keys::REGION_ID).unwrap();
					let region_id = Uuid::parse_str(region_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &region_id)
							.await?
							.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(check_region_status)),
		],
	);

	// Add a new region
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::region::ADD,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(add_region)),
		],
	);

	// Delete a new region
	app.delete(
		"/:regionId",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::region::DELETE,
				resource: closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let region_id =
						context.get_param(request_keys::REGION_ID).unwrap();
					let region_id = Uuid::parse_str(region_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &region_id)
							.await?
							.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(delete_region)),
		],
	);

	app
}

async fn list_regions(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: ListRegionsForWorkspacePath { workspace_id },
		query: Paginated {
			count: _,
			start: _,
			query: (),
		},
		body: (),
	}: DecodedRequest<ListRegionsForWorkspaceRequest>,
) -> Result<ListRegionsForWorkspaceResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - Listing all regions", request_id);

	let regions =
		db::get_all_regions_for_workspace(&mut connection, &workspace_id)
			.await?
			.into_iter()
			.map(|region| Region {
				r#type: if region.is_byoc_region() {
					RegionType::BYOC
				} else {
					RegionType::PatrOwned
				},
				id: region.id,
				name: region.name,
				cloud_provider: region.cloud_provider,
				status: region.status,
			})
			.collect();

	log::trace!("request_id: {} - Returning regions", request_id);
	Ok(ListRegionsForWorkspaceResponse { regions })
}

async fn get_region(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GetRegionInfoPath {
			workspace_id,
			region_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<GetRegionInfoRequest>,
) -> Result<GetRegionInfoResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - Listing all regions", request_id);

	let region = db::get_region_by_id(&mut connection, &region_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	log::trace!("request_id: {} - Returning region", request_id);
	Ok(GetRegionInfoResponse {
		region: Region {
			r#type: if region.is_byoc_region() {
				RegionType::BYOC
			} else {
				RegionType::PatrOwned
			},
			id: region.id,
			name: region.name,
			cloud_provider: region.cloud_provider,
			status: region.status,
		},
		disconnected_at: region.disconnected_at.map(DateTime),
		message_log: region.message_log,
	})
}

async fn check_region_status(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: CheckRegionStatusPath {
			workspace_id,
			region_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<CheckRegionStatusRequest>,
) -> Result<CheckRegionStatusResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - Check region status", request_id);

	let region = db::get_region_by_id(&mut connection, &region_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	match (
		region.status.clone(),
		region.config_file,
		region.ingress_hostname,
	) {
		(
			RegionStatus::Disconnected | RegionStatus::Active,
			Some(Json(kubeconfig)),
			Some(prev_ingress_hostname),
		) => {
			let curr_ingress_hostname =
				service::get_patr_ingress_load_balancer_hostname(kubeconfig)
					.await;
			let is_connected = match curr_ingress_hostname {
				Ok(Some(curr_ingress_hostname))
					if curr_ingress_hostname.to_string() ==
						prev_ingress_hostname =>
				{
					true
				}
				invalid_cases => {
					log::info!(
						"Invalid cases found while fetching status for region {} - {:?}",
						region_id,
						invalid_cases
					);
					false
				}
			};

			if region.status == RegionStatus::Active && !is_connected {
				log::info!("Marking the cluster {region_id} as disconnected");
				db::set_region_as_disconnected(
					&mut connection,
					&region_id,
					&Utc::now(),
				)
				.await?;
			} else if region.status == RegionStatus::Disconnected &&
				is_connected
			{
				log::info!(
					"Region `{}` got connected again. So marking it as active",
					region_id
				);
				db::set_region_as_connected(&mut connection, &region_id)
					.await?;
			}
		}
		_ => {
			log::info!("The cluster {} is not in expected state, so skipping to check status", region_id);
		}
	}

	let region = db::get_region_by_id(&mut connection, &region_id)
		.await?
		.status(500)?;

	log::trace!("request_id: {} - Returning check region status", request_id);
	Ok(CheckRegionStatusResponse {
		region: Region {
			r#type: if region.is_byoc_region() {
				RegionType::BYOC
			} else {
				RegionType::PatrOwned
			},
			id: region.id,
			name: region.name,
			cloud_provider: region.cloud_provider,
			status: region.status,
		},
		disconnected_at: region.disconnected_at.map(DateTime),
		message_log: region.message_log,
	})
}

async fn add_region(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: AddRegionToWorkspacePath { workspace_id },
		query: (),
		body: AddRegionToWorkspaceRequest { name, data },
	}: DecodedRequest<AddRegionToWorkspaceRequest>,
) -> Result<AddRegionToWorkspaceResponse, Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {} - Checking if the deployment name already exists",
		request_id
	);
	let existing_region = db::get_region_by_name_in_workspace(
		&mut connection,
		&name,
		&workspace_id,
	)
	.await?;
	if existing_region.is_some() {
		return Err(ErrorType::ResourceExists.into());
	}

	log::trace!(
		"{} - Adding new region to workspace {}",
		request_id,
		workspace_id,
	);

	let region_id = db::generate_new_resource_id(&mut connection).await?;

	db::create_resource(
		&mut connection,
		&region_id,
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(crate::models::rbac::resource_types::DEPLOYMENT_REGION)
			.unwrap(),
		&workspace_id,
		&Utc::now(),
	)
	.await?;

	match data {
		AddRegionToWorkspaceData::Digitalocean {
			region,
			api_token,
			cluster_name,
			min_nodes,
			max_nodes,
			auto_scale,
			node_name,
			node_size_slug,
		} => {
			log::trace!(
				"request_id: {} creating digital ocean k8s cluster in db",
				request_id
			);

			let cf_cert = service::create_origin_ca_certificate_for_region(
				&region_id, &config,
			)
			.await?;

			db::add_region_to_workspace(
				&mut connection,
				&region_id,
				&name,
				&InfrastructureCloudProvider::Digitalocean,
				&workspace_id,
				&cf_cert.id,
			)
			.await?;

			let cluster_id = service::create_do_k8s_cluster(
				&region.to_string(),
				&api_token,
				&cluster_name,
				min_nodes,
				max_nodes,
				auto_scale,
				&node_name,
				&node_size_slug,
				&request_id,
			)
			.await?;

			connection.commit().await?;

			log::trace!(
				"reqeust_id: {} successfully got k8s ID: {}",
				request_id,
				cluster_id
			);

			service::queue_get_kube_config_for_do_cluster(
				&api_token,
				&cluster_id,
				&region_id,
				&cf_cert.cert,
				&cf_cert.key,
				&config,
				&request_id,
			)
			.await?;
		}
		AddRegionToWorkspaceData::KubeConfig { config_file } => {
			let cf_cert = service::create_origin_ca_certificate_for_region(
				&region_id, &config,
			)
			.await?;

			db::add_region_to_workspace(
				&mut connection,
				&region_id,
				&name,
				&InfrastructureCloudProvider::Other,
				&workspace_id,
				&cf_cert.id,
			)
			.await?;

			connection.commit().await?;

			service::queue_setup_kubernetes_cluster(
				&region_id,
				config_file,
				&cf_cert.cert,
				&cf_cert.key,
				&config,
				&request_id,
			)
			.await?;
		}
	}

	log::trace!(
		"request_id: {} - Successfully added region to workspace",
		request_id
	);
	Ok(AddRegionToWorkspaceResponse { region_id })
}

async fn delete_region(
	mut connection: Connection,
	ClientIp(ip_address): ClientIp,
	Extension(token_data): Extension<UserAuthenticationData>,
	State(config): State<Config>,
	DecodedRequest {
		path: DeleteRegionFromWorkspacePath {
			workspace_id,
			region_id,
		},
		query: DeleteRegionFromWorkspaceRequest { hard_delete },
		body: (),
	}: DecodedRequest<DeleteRegionFromWorkspaceRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	let user_id = token_data.user_id();
	let login_id = token_data.login_id();

	log::trace!(
		"{} - requested to delete region: {} from workspace {}",
		request_id,
		region_id,
		workspace_id,
	);

	let region = db::get_region_by_id(&mut connection, &region_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	if region.status == RegionStatus::Deleted {
		return Err(ErrorType::NotFound.into());
	}

	log::trace!(
		"request_id: {} - check whether this region {} is associated with any ci-runner",
		request_id,
		region_id
	);
	let runners_in_region =
		db::get_runners_for_workspace(&mut connection, &workspace_id)
			.await?
			.into_iter()
			.filter(|runner| runner.region_id == region_id)
			.collect::<Vec<_>>();

	if !runners_in_region.is_empty() {
		log::trace!(
			"request_id: {} is getting all the deployment for region: {}",
			request_id,
			region_id
		);
		return Err(ErrorType::ResourceInUse.into());
	}

	log::trace!(
		"request_id: {} - check whether this region {} is used by any deployments",
		request_id,
		region_id
	);
	let deployments = db::get_deployments_by_region_id(
		&mut connection,
		&workspace_id,
		&region_id,
	)
	.await?;

	for deployment in &deployments {
		service::delete_deployment(
			&mut connection,
			&deployment.workspace_id,
			&deployment.id,
			&region_id,
			Some(&user_id),
			Some(&login_id),
			&ip_address,
			false,
			hard_delete,
			&config,
			&request_id,
		)
		.await?
	}

	// delete origin ca cert
	if let Some(cert_id) = region.cloudflare_certificate_id {
		service::revoke_origin_ca_certificate(&cert_id, &config).await?;
	}

	let onpatr_domain = db::get_domain_by_name(
		&mut connection,
		&config.cloudflare.onpatr_domain,
	)
	.await?
	.status(500)?;

	let byoc_dns_record_name = format!("*.{}", region_id);
	let byoc_dns_record =
		db::get_dns_records_by_domain_id(&mut connection, &onpatr_domain.id)
			.await?
			.into_iter()
			.find(|dns| dns.name == byoc_dns_record_name);

	if let Some(byoc_dns_record) = byoc_dns_record {
		service::delete_patr_domain_dns_record(
			&mut connection,
			&onpatr_domain.id,
			&byoc_dns_record.id,
			&config,
			&request_id,
		)
		.await?;
	}

	if hard_delete {
		service::queue_delete_kubernetes_cluster(
			&region_id,
			&workspace_id,
			region.config_file.unwrap_or_default().0,
			&config,
			&request_id,
		)
		.await?;
	}

	db::delete_region(&mut connection, &region_id, &Utc::now()).await?;

	Ok(())
}
