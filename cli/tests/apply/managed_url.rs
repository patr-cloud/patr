//! `patr apply` against managed-URL resources.

use std::collections::BTreeMap;

use cli::prelude::*;
use models::api::workspace::{deployment::*, domain::*, managed_url::*};
use wiremock::{
	Mock,
	MockServer,
	matchers::{method, path},
};

use super::*;
use crate::setup;

const CONFIG: &str = r#"
- type: ManagedUrl
  sub_domain: "app"
  domain: "example.com"
  path: "/"
  to: deployment
  deployment: "test-deployment"
  port: 8080
"#;

struct Ids {
	workspace: Uuid,
	domain: Uuid,
	deployment: Uuid,
	managed_url: Uuid,
	runner: Uuid,
	machine_type: Uuid,
}

impl Ids {
	fn new() -> Self {
		Self {
			workspace: Uuid::parse_str("00000000000000000000000000000001").unwrap(),
			domain: Uuid::parse_str("00000000000000000000000000000011").unwrap(),
			deployment: Uuid::parse_str("00000000000000000000000000000012").unwrap(),
			managed_url: Uuid::parse_str("00000000000000000000000000000013").unwrap(),
			runner: Uuid::parse_str("00000000000000000000000000000014").unwrap(),
			machine_type: Uuid::parse_str("00000000000000000000000000000015").unwrap(),
		}
	}
}

/// Mount the domain, deployment and deployment-info lookups every managed-URL
/// apply performs. `ports` is what the target deployment exposes.
async fn mount_lookups(ids: &Ids, ports: Vec<u16>) -> &'static MockServer {
	let server = setup::reset().await;

	Mock::given(method("GET"))
		.and(path(format!("/workspace/{}/domain", ids.workspace)))
		.respond_with(setup::success_list(
			ListDomainsInWorkspaceResponse {
				domains: vec![WithId::new(
					ids.domain,
					WorkspaceDomain {
						name: "example.com".to_string(),
						last_verified: None,
						is_verified: true,
					},
				)],
			},
			1,
		))
		.mount(server)
		.await;

	let deployment = WithId::new(
		ids.deployment,
		external_deployment("test-deployment", ids.runner, ids.machine_type, "1.27"),
	);

	mount_deployment_list(server, ids.workspace, vec![deployment.clone()]).await;
	mount_deployment_info(
		server,
		ids.workspace,
		deployment,
		DeploymentRunningDetails {
			deploy_on_push: false,
			min_horizontal_scale: 1,
			max_horizontal_scale: 1,
			ports: ports
				.into_iter()
				.map(|port| (StringifiedU16::new(port), ExposedPortType::Http))
				.collect(),
			environment_variables: BTreeMap::new(),
			startup_probe: None,
			liveness_probe: None,
			config_mounts: BTreeMap::new(),
			volumes: BTreeMap::new(),
		},
	)
	.await;

	server
}

/// Mount `GET /workspace/{id}/infrastructure/managed-url`.
async fn mount_managed_url_list(server: &MockServer, ids: &Ids, urls: Vec<WithId<ManagedUrl>>) {
	let total = urls.len();

	Mock::given(method("GET"))
		.and(path(format!(
			"/workspace/{}/infrastructure/managed-url",
			ids.workspace
		)))
		.respond_with(setup::success_list(ListManagedURLResponse { urls }, total))
		.mount(server)
		.await;
}

fn existing_managed_url(ids: &Ids) -> WithId<ManagedUrl> {
	WithId::new(
		ids.managed_url,
		ManagedUrl {
			sub_domain: "app".to_string(),
			domain_id: ids.domain,
			path: "/".to_string(),
			url_type: ManagedUrlType::ProxyDeployment {
				deployment_id: ids.deployment,
				port: 8080,
			},
			is_active: true,
		},
	)
}

/// With no matching managed URL, apply creates one.
#[tokio::test]
async fn create_when_none_matches() {
	let ids = Ids::new();
	let server = mount_lookups(&ids, vec![8080]).await;

	mount_managed_url_list(server, &ids, vec![]).await;
	Mock::given(method("POST"))
		.and(path(format!(
			"/workspace/{}/infrastructure/managed-url",
			ids.workspace
		)))
		.respond_with(setup::success(CreateManagedURLResponse {
			id: OnlyId::only_id(ids.managed_url),
		}))
		.mount(server)
		.await;

	setup::apply(setup::state(ids.workspace), CONFIG, &[])
		.await
		.expect("apply failed");

	let body = sole_body::<CreateManagedURLRequest>(
		server,
		"POST",
		&format!("/workspace/{}/infrastructure/managed-url", ids.workspace),
	)
	.await;

	assert_eq!(body.sub_domain, "app");
	assert_eq!(body.domain_id, ids.domain);
	assert_eq!(
		body.url_type,
		ManagedUrlType::ProxyDeployment {
			deployment_id: ids.deployment,
			port: 8080,
		}
	);
}

/// An existing managed URL is updated in place.
#[tokio::test]
async fn update_when_one_matches() {
	let ids = Ids::new();
	let server = mount_lookups(&ids, vec![8080]).await;

	mount_managed_url_list(server, &ids, vec![existing_managed_url(&ids)]).await;
	Mock::given(method("POST"))
		.and(path(format!(
			"/workspace/{}/infrastructure/managed-url/{}",
			ids.workspace, ids.managed_url
		)))
		.respond_with(setup::success(UpdateManagedURLResponse))
		.mount(server)
		.await;

	setup::apply(setup::state(ids.workspace), CONFIG, &[])
		.await
		.expect("apply failed");

	let body = sole_body::<UpdateManagedURLRequest>(
		server,
		"POST",
		&format!(
			"/workspace/{}/infrastructure/managed-url/{}",
			ids.workspace, ids.managed_url
		),
	)
	.await;

	assert_eq!(body.path, "/");
	assert_eq!(
		body.url_type,
		ManagedUrlType::ProxyDeployment {
			deployment_id: ids.deployment,
			port: 8080,
		}
	);
}

/// Pointing at a port the deployment doesn't expose is an error, and the
/// message lists what is available.
#[tokio::test]
async fn rejects_a_port_the_deployment_does_not_expose() {
	let ids = Ids::new();
	let server = mount_lookups(&ids, vec![3000]).await;

	mount_managed_url_list(server, &ids, vec![]).await;

	let error = setup::apply(setup::state(ids.workspace), CONFIG, &[])
		.await
		.expect_err("apply should have rejected the unexposed port");

	let message = error.to_string();
	assert!(
		message.contains("3000"),
		"the error should list the available ports, got: {message}"
	);

	assert_no_writes(server).await;
}

/// A dry run writes nothing.
#[tokio::test]
async fn dry_run_does_not_write() {
	let ids = Ids::new();
	let server = mount_lookups(&ids, vec![8080]).await;

	mount_managed_url_list(server, &ids, vec![existing_managed_url(&ids)]).await;

	setup::apply(setup::state(ids.workspace), CONFIG, &["--dry-run"])
		.await
		.expect("dry run failed");

	assert_no_writes(server).await;
}
