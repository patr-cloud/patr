//! Tests for what the Docker runner expects for each owner: the routes it
//! renders into Caddy and the spec it hashes for a deployment's service.

use std::collections::BTreeMap;

use docker::prelude::*;
use models::api::workspace::{deployment::*, runner::RunnerExposureType};
use time::OffsetDateTime;

/// A deployment ID that never changes, so the golden spec is stable.
const DEPLOYMENT_ID: &str = "0f9e8d7c6b5a49382716051f2e3d4c5b";

/// A runner with the given exposure type. Its Docker client is never used:
/// `expected` doesn't touch Docker.
fn runner(runner_exposure_type: RunnerExposureType) -> DockerRunner {
	DockerRunner {
		docker: bollard::Docker::connect_with_defaults().unwrap(),
		settings: RunnerSettings {
			workspace_id: Uuid::parse_str("6a1e0c9f5b0d4f0e8d2a4b7c9e1f3a5b").unwrap(),
			runner_id: Uuid::parse_str("1a2b3c4d5e6f40718293a4b5c6d7e8f9").unwrap(),
			api_token: "test-token".parse().unwrap(),
			user_agent: "integration-tests/0.1.0".parse().unwrap(),
			environment: RunningEnvironment::Development,
			database: DatabaseConfig {
				file: String::from("unused.db"),
				connection_limit: 1,
			},
			data: DockerSettings {
				docker_swarm_listen_addr: String::from("127.0.0.1:2377"),
				ingress_http_listen_port: 80,
				ingress_https_listen_port: 443,
				runner_exposure_type,
				enable_ipv6: true,
			},
		},
		platforms: Default::default(),
	}
}

/// A deployment with an HTTP port, a plain and a secret environment variable,
/// a config mount and a volume.
fn deployment(should_run: bool) -> (Deployment, DeploymentRunningDetails) {
	(
		Deployment {
			name: String::from("app"),
			registry: DeploymentRegistry::ExternalRegistry {
				registry: String::from("docker.io"),
				image_name: String::from("library/nginx"),
			},
			image_tag: String::from("1.27"),
			status: DeploymentStatus::Observed(ObservedDeploymentStatus::Unknown),
			should_run,
			runner: Uuid::nil(),
			current_live_digest: None,
			machine_type: Uuid::parse_str("b3cf3771fa394281bfdfeb2e65a061b6").unwrap(),
		},
		DeploymentRunningDetails {
			deploy_on_push: false,
			min_horizontal_scale: 2,
			max_horizontal_scale: 4,
			ports: BTreeMap::from([(StringifiedU16::new(8080), ExposedPortType::Http)]),
			environment_variables: BTreeMap::from([
				(
					String::from("MODE"),
					EnvironmentVariableValue::String(String::from("production")),
				),
				(
					String::from("TOKEN"),
					EnvironmentVariableValue::Secret {
						from_secret: Uuid::parse_str("2b3c4d5e6f7a48190a1b2c3d4e5f6a7b").unwrap(),
					},
				),
			]),
			startup_probe: None,
			liveness_probe: Some(DeploymentProbe {
				port: 8080,
				path: String::from("/health"),
			}),
			config_mounts: BTreeMap::from([(
				String::from("/etc/app.conf"),
				Base64String::from(b"key=value".to_vec()),
			)]),
			volumes: BTreeMap::from([(String::from("/data"), VolumeConfig {})]),
		},
	)
}

/// The local state holding `deployment` under [`DEPLOYMENT_ID`], the secret it
/// references and a managed URL at `host` and `path` pointing at it.
fn local(should_run: bool, host: &str, path: &str) -> (LocalState, Uuid, Uuid) {
	let deployment_id = Uuid::parse_str(DEPLOYMENT_ID).unwrap();
	let managed_url_id = Uuid::parse_str("3c4d5e6f7a8b49201b2c3d4e5f6a7b8c").unwrap();
	let local = LocalState {
		deployments: BTreeMap::from([(deployment_id, deployment(should_run))]),
		managed_urls: BTreeMap::from([(
			managed_url_id,
			LocalManagedUrl {
				host: String::from(host),
				path: String::from(path),
				deployment_id,
				port: 8080,
			},
		)]),
		secrets: BTreeMap::from([(
			Uuid::parse_str("2b3c4d5e6f7a48190a1b2c3d4e5f6a7b").unwrap(),
			OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
		)]),
	};
	(local, deployment_id, managed_url_id)
}

/// The content of the one config in a graph, as text.
fn only_config(graph: &ExpectedGraph<DockerObjectId, DockerSpec>) -> String {
	let mut configs = graph.nodes.values().filter_map(|(spec, _)| match spec {
		DockerSpec::Config { data } => Some(String::from_utf8(data.clone()).unwrap()),
		_ => None,
	});
	let config = configs.next().expect("the graph has no config");
	assert!(
		configs.next().is_none(),
		"the graph has more than one config"
	);
	config
}

/// Every service an owner expects traces back to that owner from its name,
/// since that's all Docker's events carry, and no other service does.
#[test]
fn services_trace_back_to_their_owner() {
	let runner = runner(RunnerExposureType::Private);
	let (local, deployment_id, _) = local(true, "app.example.com", "/");

	for (owner, inputs) in [
		(Owner::Deployment(deployment_id), None),
		(Owner::System, Some(String::from("tunnel-token-hash"))),
	] {
		let services = runner
			.expected(owner, &local, &inputs)
			.unwrap()
			.nodes
			.into_keys()
			.filter_map(|id| match id {
				DockerObjectId::Service(name) => Some(name),
				_ => None,
			})
			.collect::<Vec<_>>();
		assert!(!services.is_empty(), "{owner} expects no service");
		for name in services {
			assert_eq!(DockerRunner::service_owner(&name), Some(owner), "{name}");
		}
	}

	assert_eq!(DockerRunner::service_owner("patr-not-a-deployment"), None);
	assert_eq!(DockerRunner::service_owner("someone-elses-service"), None);
}

/// A managed URL's route points at its deployment, matches every path under
/// its own, and serves Patr's error pages.
#[test]
fn a_managed_url_routes_to_its_deployment() {
	let runner = runner(RunnerExposureType::Private);
	let (local, deployment_id, managed_url_id) = local(true, "app.example.com", "/api");

	let graph = runner
		.expected(Owner::ManagedUrl(managed_url_id), &local, &None)
		.unwrap();
	let route = only_config(&graph);

	assert!(route.contains("http://app.example.com/api* {"));
	assert!(route.contains("import patr_errors"));
	assert!(route.contains(&format!("reverse_proxy {deployment_id}.onpatr.local:8080")));
}

/// A host or path that could break out of a Caddyfile site address is refused,
/// which fails only that managed URL.
#[test]
fn unsafe_managed_url_hosts_and_paths_are_refused() {
	let runner = runner(RunnerExposureType::Private);
	for (host, path) in [
		("app.example.com", "/x {"),
		("app.example.com", "/x\n}\nevil.com {"),
		("app.example.com", "/a#b"),
		("app.example.com", "/a b"),
		("app.example.com", "/{env.SECRET}"),
		("app.example.com", "/a\"b"),
		("app.example.com", "/a\\b"),
		("app.example.com, evil.com", "/"),
		("app.example.com {", "/"),
		("app example.com", "/"),
		("*.example.com", "/"),
		("app.example.com\n", "/"),
	] {
		let (local, _, managed_url_id) = local(true, host, path);
		assert!(
			runner
				.expected(Owner::ManagedUrl(managed_url_id), &local, &None)
				.is_err(),
			"{host:?} {path:?} was rendered"
		);
	}
}

/// A stopped deployment keeps its volume and routes, the routes answer 503,
/// and it has no service.
#[test]
fn a_stopped_deployment_answers_503() {
	let runner = runner(RunnerExposureType::Private);
	let (local, deployment_id, managed_url_id) = local(false, "app.example.com", "/");

	let graph = runner
		.expected(Owner::Deployment(deployment_id), &local, &None)
		.unwrap();
	assert!(only_config(&graph).contains("error 503"));
	assert!(
		graph
			.nodes
			.keys()
			.any(|id| matches!(id, DockerObjectId::Volume(_)))
	);
	assert!(
		!graph
			.nodes
			.keys()
			.any(|id| matches!(id, DockerObjectId::Service(_)))
	);

	let graph = runner
		.expected(Owner::ManagedUrl(managed_url_id), &local, &None)
		.unwrap();
	assert!(only_config(&graph).contains("error 503"));
}

/// The ingress imports every deployment's and managed URL's routes, and a
/// private runner only runs a tunnel once it has a token.
#[test]
fn the_ingress_imports_every_route() {
	let runner = runner(RunnerExposureType::Private);
	let (local, deployment_id, managed_url_id) = local(true, "app.example.com", "/");

	let route_of = |owner, prefix: &str| {
		runner
			.expected(owner, &local, &None)
			.unwrap()
			.nodes
			.into_keys()
			.find(|id| matches!(id, DockerObjectId::Config(name) if name.starts_with(prefix)))
			.unwrap()
	};
	let deployment_route = route_of(Owner::Deployment(deployment_id), "patr-ingress-");
	let managed_url_route = route_of(Owner::ManagedUrl(managed_url_id), "patr-url-");

	let system = runner.expected(Owner::System, &local, &None).unwrap();
	let Some((DockerSpec::Service(ingress), _)) = system.nodes.get(&DockerObjectId::Service(
		String::from(constants::INGRESS_SERVICE_NAME),
	)) else {
		panic!("the runner expects no ingress");
	};
	let ServiceTemplate::Ingress(ingress) = ingress.as_ref() else {
		panic!("the ingress isn't an ingress");
	};
	assert!(ingress.imports.contains(&deployment_route));
	assert!(ingress.imports.contains(&managed_url_route));
	assert!(
		!system
			.nodes
			.contains_key(&DockerObjectId::Service(String::from(
				constants::TUNNEL_SERVICE_NAME
			)))
	);

	let system = runner
		.expected(Owner::System, &local, &Some(String::from("token-hash")))
		.unwrap();
	assert!(
		system
			.nodes
			.contains_key(&DockerObjectId::Service(String::from(
				constants::TUNNEL_SERVICE_NAME
			)))
	);
}

/// The deployment service's spec serializes exactly as before. Its JSON is
/// hashed into `patr.specHash`, so any change here, even a serde rename,
/// re-applies every deployment on every runner. If the change is intended,
/// bump `SPEC_VERSION` too and update this string.
#[test]
fn the_deployment_service_spec_is_stable() {
	let runner = runner(RunnerExposureType::Private);
	let (local, deployment_id, _) = local(true, "app.example.com", "/");

	let graph = runner
		.expected(Owner::Deployment(deployment_id), &local, &None)
		.unwrap();
	let (spec, _) = &graph.nodes[&DockerObjectId::Service(format!("patr-{deployment_id}"))];

	assert_eq!(serde_json::to_string(spec).unwrap(), GOLDEN_SERVICE_SPEC);
}

/// The serialized spec of the deployment in [`deployment`].
const GOLDEN_SERVICE_SPEC: &str = r#"{"service":{"deployment":{"deploymentId":"0f9e8d7c6b5a49382716051f2e3d4c5b","registry":{"registry":"docker.io","imageName":"library/nginx"},"imageTag":"1.27","currentLiveDigest":null,"env":{"MODE":{"plain":"production"},"TOKEN":{"secret":{"secretId":"2b3c4d5e6f7a48190a1b2c3d4e5f6a7b","lastUpdated":"2023-11-14 22:13:20.0 +00:00:00"}}},"replicas":2,"startupProbe":null,"livenessProbe":{"port":8080,"path":"/health"},"configMounts":{"/etc/app.conf":{"kind":"config","name":"patr-config-0f9e8d7c6b5a49382716051f2e3d4c5b-563f0357118d05ef"}},"volumeMounts":{"/data":{"kind":"volume","name":"patr-0f9e8d7c6b5a49382716051f2e3d4c5b-bd47413b5c03dfc6"}},"updateOrder":"stop-first"}}}"#;
