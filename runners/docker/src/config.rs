use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// The configuration for the runner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerSettings {
	/// The address to listen on for Docker Swarm. If not provided, the default
	/// Docker Swarm listen address will be used.
	#[serde(default = "default_docker_swarm_listen_addr")]
	pub docker_swarm_listen_addr: String,
	/// The port the ingress should listen on for HTTP connections.
	/// Defaults to `80`.
	#[serde(default = "default_ingress_http_listen_port")]
	pub ingress_http_listen_port: u16,
	/// The port the ingress should listen on for HTTPS connections.
	/// Defaults to `443`.
	#[serde(default = "default_ingress_https_listen_port")]
	pub ingress_https_listen_port: u16,
	/// The exposure type for the runner.
	#[serde(default = "default_runner_exposure_type")]
	pub runner_exposure_type: RunnerExposureType,
}

/// The default Docker Swarm listen address. Defaults to `127.0.0.1:2377`.
fn default_docker_swarm_listen_addr() -> String {
	String::from("127.0.0.1:2377")
}

/// The default ingress listen port. Defaults to `80`.
fn default_ingress_http_listen_port() -> u16 {
	80
}

/// The default ingress HTTPS listen port. Defaults to `443`.
fn default_ingress_https_listen_port() -> u16 {
	443
}

/// The default runner exposure type. This is `Private`, meaning that the runner
/// will not be exposed to the public internet by default.
fn default_runner_exposure_type() -> RunnerExposureType {
	RunnerExposureType::Private
}
