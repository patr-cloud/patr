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
}
