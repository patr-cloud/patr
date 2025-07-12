/// Constants used in the Worker
pub mod constants {
	/// The default domain for the PATR platform. Any requests to this domain
	/// will be either a deployment or a static site that has the default domain
	pub const DEFAULT_PATR_DOMAIN: &str = "onpatr.cloud";

	/// The cloudflare KV namespace that stores the ingress configuration
	pub const INGRESS_KV: &str = "INGRESS_KV";
	/// The cloudflare R2 bucket that stores all the static sites
	pub const STATIC_SITE_BUCKET: &str = "STATIC_SITE_BUCKET";

	/// The default status code for a temporary redirect
	pub const STATUS_CODE_TEMPORAL_REDIRECT: u16 = 307;
	/// The default status code for a permanent redirect
	pub const STATUS_CODE_PERMANENT_REDIRECT: u16 = 308;
}
