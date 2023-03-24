#[cfg(test)]
mod tests {
	use api_models::{
		models::workspace::{
			domain::{AddDomainRequest, DomainNameserverType},
			infrastructure::{
				deployment::{DeploymentStatus, ExposedPortType},
				managed_urls::{
					CreateNewManagedUrlRequest,
					ManagedUrlType as IdManagedUrlType,
				},
				static_site::{CreateStaticSiteRequest, StaticSiteDetails},
			},
			region::RegionStatus,
		},
		utils::{ResourceType, Uuid},
	};
	use chrono::Utc;

	use crate::{
		db::*,
		models::rbac,
		test::{deinit_test, user_constaints},
		utils::Error,
	};

	#[tokio::test]
	async fn managed_url_test() -> Result<(), Error> {
		let app = user_constaints().await?;
		let mut connection = app.database.acquire().await?;
		let workspace_id =
			get_workspace_by_name(&mut connection, "test-workspace")
				.await?
				.unwrap()
				.id;

		// Create static managed url
		let static_site_id = generate_new_resource_id(&mut connection).await?;
		let static_site = CreateStaticSiteRequest {
			workspace_id: workspace_id.clone(),
			name: "test-site".to_string(),
			message: "v1".to_string(),
			file: Some("/test".to_owned()),
			static_site_details: StaticSiteDetails {},
		};

		create_resource(
			&mut connection,
			&static_site_id,
			rbac::RESOURCE_TYPES
				.get()
				.unwrap()
				.get(rbac::resource_types::STATIC_SITE)
				.unwrap(),
			&workspace_id,
			&Utc::now(),
		)
		.await?;
		create_static_site(
			&mut connection,
			&static_site_id,
			&static_site.name,
			&workspace_id,
		)
		.await?;
		let domain = AddDomainRequest {
			workspace_id: workspace_id.clone(),
			domain: "test.com".to_string(),
			nameserver_type: DomainNameserverType::External,
		};
		let domain_id = generate_new_domain_id(&mut connection).await?;
		let cloudflare_worker_route_id = Uuid::new_v4();
		create_resource(
			&mut connection,
			&domain_id,
			rbac::RESOURCE_TYPES
				.get()
				.unwrap()
				.get(rbac::resource_types::DOMAIN)
				.unwrap(),
			&workspace_id,
			&Utc::now(),
		)
		.await?;
		sqlx::query("INSERT INTO domain_tld VALUES ('com.au')")
			.execute(&mut connection)
			.await?;
		create_generic_domain(
			&mut connection,
			&domain_id,
			"test",
			"com.au",
			&ResourceType::Business,
		)
		.await?;
		add_to_workspace_domain(
			&mut connection,
			&domain_id,
			&domain.nameserver_type,
			&cloudflare_worker_route_id.to_string(),
		)
		.await?;
		add_user_controlled_domain(&mut connection, &domain_id).await?;

		let static_managed_url = CreateNewManagedUrlRequest {
			workspace_id: workspace_id.clone(),
			sub_domain: "subtestone".to_string(),
			domain_id: domain_id.clone(),
			path: "/test".to_string(),
			url_type: IdManagedUrlType::ProxyStaticSite {
				static_site_id: static_site_id.clone(),
			},
		};
		let static_managed_url_id =
			generate_new_resource_id(&mut connection).await?;
		create_resource(
			&mut connection,
			&static_managed_url_id,
			rbac::RESOURCE_TYPES
				.get()
				.unwrap()
				.get(rbac::resource_types::MANAGED_URL)
				.unwrap(),
			&workspace_id,
			&Utc::now(),
		)
		.await?;
		let cloudflare_custom_hostname_id = Uuid::new_v4();
		create_new_managed_url_in_workspace(
			&mut connection,
			&static_managed_url_id,
			&static_managed_url.sub_domain,
			&static_managed_url.domain_id,
			&static_managed_url.path,
			&ManagedUrlType::ProxyToStaticSite,
			None,
			None,
			Some(&static_site_id),
			None,
			&workspace_id,
			false,
			None,
			None,
			cloudflare_custom_hostname_id.to_string(),
		)
		.await?;

		println!("creating static done");

		// Create deployment managed url
		let region_id = get_all_default_regions(&mut connection)
			.await?
			.into_iter()
			.find(|region| region.status == RegionStatus::Active)
			.unwrap()
			.id;
		let machine =
			&get_all_deployment_machine_types(&mut connection).await?[0];

		let deployment = Deployment {
			id: Uuid::new_v4(),
			name: "test".to_owned(),
			registry: "docker.io".to_owned(),
			repository_id: None,
			image_name: Some("test_image".to_owned()),
			image_tag: "test_image".to_owned(),
			status: DeploymentStatus::Created,
			workspace_id: workspace_id.clone(),
			region: region_id,
			min_horizontal_scale: 1,
			max_horizontal_scale: 2,
			machine_type: machine.id.clone(),
			deploy_on_push: true,
			startup_probe_path: Some("/data".to_owned()),
			startup_probe_port: Some(2020),
			liveness_probe_path: Some("/data".to_owned()),
			liveness_probe_port: Some(2020),
			current_live_digest: Some("test_digest".to_owned()),
		};
		create_resource(
			&mut connection,
			&deployment.id,
			rbac::RESOURCE_TYPES
				.get()
				.unwrap()
				.get(rbac::resource_types::DEPLOYMENT)
				.unwrap(),
			&workspace_id,
			&Utc::now(),
		)
		.await?;

		// Creating a deployment
		create_deployment_with_external_registry(
			&mut connection,
			&deployment.id,
			&deployment.name,
			&deployment.registry,
			&deployment.image_name.clone().unwrap().as_str(),
			&deployment.image_tag,
			&deployment.workspace_id,
			&deployment.region,
			&deployment.machine_type,
			deployment.deploy_on_push,
			deployment.min_horizontal_scale as u16,
			deployment.max_horizontal_scale as u16,
			None,
			None,
		)
		.await?;
		add_exposed_port_for_deployment(
			&mut connection,
			&deployment.id,
			1000,
			&ExposedPortType::Http,
		)
		.await?;
		add_exposed_port_for_deployment(
			&mut connection,
			&deployment.id,
			2020,
			&ExposedPortType::Http,
		)
		.await?;
		let deployment_managed_url = CreateNewManagedUrlRequest {
			workspace_id: workspace_id.clone(),
			sub_domain: "subtesttwo".to_string(),
			domain_id: domain_id.clone(),
			path: "/test".to_string(),
			url_type: IdManagedUrlType::ProxyStaticSite {
				static_site_id: static_site_id.clone(),
			},
		};
		let deployment_managed_url_id =
			generate_new_resource_id(&mut connection).await?;
		create_resource(
			&mut connection,
			&deployment_managed_url_id,
			rbac::RESOURCE_TYPES
				.get()
				.unwrap()
				.get(rbac::resource_types::MANAGED_URL)
				.unwrap(),
			&workspace_id,
			&Utc::now(),
		)
		.await?;
		create_new_managed_url_in_workspace(
			&mut connection,
			&deployment_managed_url_id,
			&deployment_managed_url.sub_domain,
			&deployment_managed_url.domain_id,
			&deployment_managed_url.path,
			&ManagedUrlType::ProxyToDeployment,
			Some(&deployment.id),
			Some(2020),
			None,
			None,
			&workspace_id,
			false,
			None,
			None,
			cloudflare_custom_hostname_id.to_string(),
		)
		.await?;

		println!("creating deployment done");

		// Create proxy managed url
		let proxy_managed_url = CreateNewManagedUrlRequest {
			workspace_id: workspace_id.clone(),
			sub_domain: "subtestthree".to_string(),
			domain_id: domain_id.clone(),
			path: "/test".to_string(),
			url_type: IdManagedUrlType::ProxyStaticSite {
				static_site_id: static_site_id.clone(),
			},
		};
		let proxy_managed_url_id =
			generate_new_resource_id(&mut connection).await?;
		create_resource(
			&mut connection,
			&proxy_managed_url_id,
			rbac::RESOURCE_TYPES
				.get()
				.unwrap()
				.get(rbac::resource_types::MANAGED_URL)
				.unwrap(),
			&workspace_id,
			&Utc::now(),
		)
		.await?;
		create_new_managed_url_in_workspace(
			&mut connection,
			&proxy_managed_url_id,
			&proxy_managed_url.sub_domain,
			&proxy_managed_url.domain_id,
			&proxy_managed_url.path,
			&ManagedUrlType::ProxyUrl,
			None,
			None,
			None,
			Some("google.com"),
			&workspace_id,
			false,
			None,
			Some(true),
			cloudflare_custom_hostname_id.to_string(),
		)
		.await?;

		println!("creating proxy done");

		// Create redirect managed url
		let redirect_managed_url = CreateNewManagedUrlRequest {
			workspace_id: workspace_id.clone(),
			sub_domain: "subtestfour".to_string(),
			domain_id: domain_id.clone(),
			path: "/test".to_string(),
			url_type: IdManagedUrlType::ProxyStaticSite {
				static_site_id: static_site_id.clone(),
			},
		};
		let redirect_managed_url_id =
			generate_new_resource_id(&mut connection).await?;
		create_resource(
			&mut connection,
			&redirect_managed_url_id,
			rbac::RESOURCE_TYPES
				.get()
				.unwrap()
				.get(rbac::resource_types::MANAGED_URL)
				.unwrap(),
			&workspace_id,
			&Utc::now(),
		)
		.await?;
		create_new_managed_url_in_workspace(
			&mut connection,
			&redirect_managed_url_id,
			&redirect_managed_url.sub_domain,
			&redirect_managed_url.domain_id,
			&redirect_managed_url.path,
			&ManagedUrlType::Redirect,
			None,
			None,
			None,
			Some("google.com"),
			&workspace_id,
			false,
			Some(true),
			Some(false),
			cloudflare_custom_hostname_id.to_string(),
		)
		.await?;

		println!("creating redirect done");

		// Test for all managed url type
		let static_managed_url_output =
			get_managed_url_by_id(&mut connection, &static_managed_url_id)
				.await?
				.unwrap();
		let deployment_managed_url_output =
			get_managed_url_by_id(&mut connection, &deployment_managed_url_id)
				.await?
				.unwrap();
		let proxy_managed_url_output =
			get_managed_url_by_id(&mut connection, &proxy_managed_url_id)
				.await?
				.unwrap();
		let redirect_managed_url_output =
			get_managed_url_by_id(&mut connection, &redirect_managed_url_id)
				.await?
				.unwrap();

		assert_eq!(static_managed_url_id, static_managed_url_output.id);
		assert_eq!(
			static_managed_url.sub_domain,
			static_managed_url_output.sub_domain
		);
		assert_eq!(static_managed_url.path, static_managed_url_output.path);
		assert_eq!(
			static_managed_url.domain_id,
			static_managed_url_output.domain_id
		);
		assert_eq!(
			static_managed_url.workspace_id,
			static_managed_url_output.workspace_id
		);
		assert_eq!(
			cloudflare_custom_hostname_id.to_string(),
			static_managed_url_output.cloudflare_custom_hostname_id
		);
		assert_eq!(None, static_managed_url_output.deployment_id);
		assert_eq!(None, static_managed_url_output.port);
		assert_eq!(None, static_managed_url_output.url);
		assert_eq!(false, static_managed_url_output.is_configured);
		assert_eq!(None, static_managed_url_output.permanent_redirect);
		assert_eq!(None, static_managed_url_output.http_only);
		assert_eq!(
			Some(static_site_id.clone()),
			static_managed_url_output.static_site_id
		);

		assert_eq!(deployment_managed_url_id, deployment_managed_url_output.id);
		assert_eq!(
			deployment_managed_url.sub_domain,
			deployment_managed_url_output.sub_domain
		);
		assert_eq!(
			deployment_managed_url.path,
			deployment_managed_url_output.path
		);
		assert_eq!(
			deployment_managed_url.domain_id,
			deployment_managed_url_output.domain_id
		);
		assert_eq!(
			deployment_managed_url.workspace_id,
			deployment_managed_url_output.workspace_id
		);
		assert_eq!(
			cloudflare_custom_hostname_id.to_string(),
			deployment_managed_url_output.cloudflare_custom_hostname_id
		);
		assert_eq!(
			deployment.id,
			deployment_managed_url_output.deployment_id.unwrap()
		);
		assert_eq!(Some(2020), deployment_managed_url_output.port);
		assert_eq!(None, deployment_managed_url_output.url);
		assert_eq!(false, deployment_managed_url_output.is_configured);
		assert_eq!(None, deployment_managed_url_output.permanent_redirect);
		assert_eq!(None, deployment_managed_url_output.http_only);
		assert_eq!(None, deployment_managed_url_output.static_site_id);

		assert_eq!(proxy_managed_url_id, proxy_managed_url_output.id);
		assert_eq!(
			proxy_managed_url.sub_domain,
			proxy_managed_url_output.sub_domain
		);
		assert_eq!(proxy_managed_url.path, proxy_managed_url_output.path);
		assert_eq!(
			proxy_managed_url.domain_id,
			proxy_managed_url_output.domain_id
		);
		assert_eq!(
			proxy_managed_url.workspace_id,
			proxy_managed_url_output.workspace_id
		);
		assert_eq!(
			cloudflare_custom_hostname_id.to_string(),
			proxy_managed_url_output.cloudflare_custom_hostname_id
		);
		assert_eq!(None, proxy_managed_url_output.deployment_id);
		assert_eq!(None, proxy_managed_url_output.port);
		assert_eq!(
			Some("google.com".to_string()),
			proxy_managed_url_output.url
		);
		assert_eq!(false, proxy_managed_url_output.is_configured);
		assert_eq!(None, proxy_managed_url_output.permanent_redirect);
		assert_eq!(Some(true), proxy_managed_url_output.http_only);
		assert_eq!(None, proxy_managed_url_output.static_site_id);

		assert_eq!(redirect_managed_url_id, redirect_managed_url_output.id);
		assert_eq!(
			redirect_managed_url.sub_domain,
			redirect_managed_url_output.sub_domain
		);
		assert_eq!(redirect_managed_url.path, redirect_managed_url_output.path);
		assert_eq!(
			redirect_managed_url.domain_id,
			redirect_managed_url_output.domain_id
		);
		assert_eq!(
			redirect_managed_url.workspace_id,
			redirect_managed_url_output.workspace_id
		);
		assert_eq!(
			cloudflare_custom_hostname_id.to_string(),
			static_managed_url_output.cloudflare_custom_hostname_id
		);
		assert_eq!(None, redirect_managed_url_output.deployment_id);
		assert_eq!(None, redirect_managed_url_output.port);
		assert_eq!(
			Some("google.com".to_string()),
			redirect_managed_url_output.url
		);
		assert_eq!(false, redirect_managed_url_output.is_configured);
		assert_eq!(Some(true), redirect_managed_url_output.permanent_redirect);
		assert_eq!(Some(false), redirect_managed_url_output.http_only);
		assert_eq!(None, redirect_managed_url_output.static_site_id);

		// Update managed url
		update_managed_url(
			&mut connection,
			&static_managed_url_id,
			"/update",
			&ManagedUrlType::ProxyToStaticSite,
			None,
			None,
			Some(&static_site_id),
			None,
			None,
			None,
		)
		.await?;

		println!("update static done");

		update_managed_url(
			&mut connection,
			&deployment_managed_url_id,
			"/update",
			&ManagedUrlType::ProxyToDeployment,
			Some(&deployment.id),
			Some(1000),
			None,
			None,
			None,
			None,
		)
		.await?;

		println!("update deployment done");

		update_managed_url(
			&mut connection,
			&proxy_managed_url_id,
			"/update",
			&ManagedUrlType::ProxyUrl,
			None,
			None,
			None,
			Some("vicara.co"),
			None,
			Some(true),
		)
		.await?;

		println!("update proxy done");

		update_managed_url(
			&mut connection,
			&redirect_managed_url_id,
			"/update",
			&ManagedUrlType::Redirect,
			None,
			None,
			None,
			Some("vicara.co"),
			Some(false),
			Some(true),
		)
		.await?;

		println!("Update redirect done");

		// Test for updation of all managed url
		let updated_static_managed_url_output =
			get_managed_url_by_id(&mut connection, &static_managed_url_id)
				.await?
				.unwrap();
		let updated_deployment_managed_url_output =
			get_managed_url_by_id(&mut connection, &deployment_managed_url_id)
				.await?
				.unwrap();
		let updated_proxy_managed_url_output =
			get_managed_url_by_id(&mut connection, &proxy_managed_url_id)
				.await?
				.unwrap();
		let updated_redirect_managed_url_output =
			get_managed_url_by_id(&mut connection, &redirect_managed_url_id)
				.await?
				.unwrap();

		assert_eq!(
			"/update".to_string(),
			updated_static_managed_url_output.path
		);
		assert_eq!(
			"/update".to_string(),
			updated_deployment_managed_url_output.path
		);
		assert_eq!(Some(1000), updated_deployment_managed_url_output.port);
		assert_eq!(
			"/update".to_string(),
			updated_proxy_managed_url_output.path
		);
		assert_eq!(
			Some("vicara.co".to_string()),
			updated_proxy_managed_url_output.url
		);
		assert_eq!(
			"/update".to_string(),
			updated_redirect_managed_url_output.path
		);
		assert_eq!(
			Some("vicara.co".to_string()),
			updated_redirect_managed_url_output.url
		);
		assert_eq!(
			Some(false),
			updated_redirect_managed_url_output.permanent_redirect
		);
		assert_eq!(Some(true), updated_redirect_managed_url_output.http_only);

		// Deleting managed url
		delete_managed_url(
			&mut connection,
			&static_managed_url_id,
			&Utc::now(),
		)
		.await?;
		delete_managed_url(
			&mut connection,
			&deployment_managed_url_id,
			&Utc::now(),
		)
		.await?;
		delete_managed_url(&mut connection, &proxy_managed_url_id, &Utc::now())
			.await?;
		delete_managed_url(
			&mut connection,
			&redirect_managed_url_id,
			&Utc::now(),
		)
		.await?;

		// Test
		let number_after_deletion =
			get_all_managed_urls_in_workspace(&mut connection, &workspace_id)
				.await?
				.len();
		assert_eq!(0, number_after_deletion);

		deinit_test(app.config.database.database).await?;
		Ok(())
	}
}
