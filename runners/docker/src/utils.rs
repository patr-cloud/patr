/// All commonly used constants in the Docker runner.
pub mod constants {
	/// The name of the patr overlay network for service discovery.
	/// NOTE: This must NOT be "ingress" - that's Docker Swarm's built-in
	/// routing mesh network which does not support DNS-based service discovery.
	pub const INGRESS_NETWORK_NAME: &str = "patr-ingress-network";
	/// The name of the ingress service.
	pub const INGRESS_SERVICE_NAME: &str = "patr-ingress";
	/// The name of the volume used to store TLS certs for the ingress service.
	pub const INGRESS_TLS_CERTS_VOLUME_NAME: &str = "patr-ingress-data";
	/// The name of the config used to store the cloudflare tunnel token.
	pub const TUNNEL_TOKEN_CONFIG_NAME: &str = "patr-tunnel-token";
	/// The name of the config used to store the ingress configuration for the
	/// runner.
	pub const INGRESS_CONFIG_NAME: &str = "patr-ingress-config";
	/// The name of the Grafana Alloy log collector service.
	pub const ALLOY_SERVICE_NAME: &str = "patr-alloy";
	/// The name of the Docker config for the Alloy configuration.
	pub const ALLOY_CONFIG_NAME: &str = "patr-alloy-config";
	/// The pinned Grafana Alloy image version.
	pub const ALLOY_IMAGE: &str = "grafana/alloy:v1.13.2";
}
