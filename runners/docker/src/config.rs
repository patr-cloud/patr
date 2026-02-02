use serde::{Deserialize, Serialize};

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
}

/// The default Docker Swarm listen address.
fn default_docker_swarm_listen_addr() -> String {
	String::from("127.0.0.1:2377")
}

/// The default ingress listen port.
fn default_ingress_http_listen_port() -> u16 {
	80
}

/// The default ingress HTTPS listen port.
fn default_ingress_https_listen_port() -> u16 {
	443
}
