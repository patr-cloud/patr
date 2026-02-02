/// All commonly used constants in the Docker runner.
pub mod constants {
	/// The name of the patr overlay network for service discovery.
	/// NOTE: This must NOT be "ingress" - that's Docker Swarm's built-in
	/// routing mesh network which does not support DNS-based service discovery.
	pub const INGRESS_NETWORK_NAME: &str = "patr-ingress-network";
	/// The name of the ingress service.
	pub const INGRESS_SERVICE_NAME: &str = "patr-ingress";
}
