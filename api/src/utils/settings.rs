use std::{
	env,
	fmt::{Display, Formatter},
	net::IpAddr,
};

use config_rs::{Config, Environment, File};
use serde::{Deserialize, Deserializer, Serialize};

pub fn parse_config() -> Settings {
	println!("[TRACE]: Reading config data...");
	let env = if cfg!(debug_assertions) {
		"dev".to_string()
	} else {
		env::var("APP_ENV").unwrap_or_else(|_| "prod".into())
	};

	match env.as_ref() {
		"prod" | "production" => Config::builder()
			.add_source(File::with_name("config/prod").required(false))
			.set_default("environment", "production")
			.expect("unable to set environment to production"),
		"dev" | "development" => Config::builder()
			.add_source(File::with_name("config/dev").required(false))
			.set_default("environment", "development")
			.expect("unable to set environment to development"),
		_ => {
			panic!("Unknown running environment found!");
		}
	}
	.add_source(Environment::with_prefix("APP").separator("_"))
	.build()
	.expect("unable to merge with environment variables")
	.try_deserialize()
	.expect("unable to parse settings")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
	pub port: u16,
	#[serde(alias = "bindaddress")]
	pub bind_address: IpAddr,
	#[serde(alias = "basepath")]
	pub base_path: String,
	#[serde(alias = "passwordpepper")]
	pub password_pepper: String,
	#[serde(alias = "jwtsecret")]
	pub jwt_secret: String,
	// Callback domain used for exposing patr webhooks/apis to internet.
	// For prod, it will be https://app.patr.cloud
	// For dev and testing, use ngrok's domain
	#[serde(alias = "frontenddomain")]
	pub frontend_domain: String,
	pub environment: RunningEnvironment,
	pub s3: S3Settings,
	pub database: DatabaseSettings,
	pub redis: RedisSettings,
	pub email: EmailSettings,
	pub twilio: TwilioSettings,
	pub cloudflare: CloudflareSettings,
	#[serde(alias = "dockerregistry")]
	pub docker_registry: DockerRegistrySettings,
	pub digitalocean: Digitalocean,
	pub kubernetes: KubernetesSettings,
	pub prometheus: PrometheusSettings,
	#[serde(alias = "rabbitmq")]
	pub rabbit_mq: RabbitMqSettings,
	pub vault: VaultSettings,
	pub loki: LokiSettings,
	pub mailchimp: MailchimpSettings,
	pub github: GithubSettings,
	pub google: GoogleSettings,
	pub drone: DroneSettings,
	pub stripe: StripeSettings,
	#[serde(alias = "ipinfotoken")]
	pub ipinfo_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct S3Settings {
	pub endpoint: String,
	pub region: String,
	pub bucket: String,
	pub key: String,
	pub secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseSettings {
	pub host: String,
	pub port: u16,
	pub user: String,
	pub password: String,
	pub database: String,
	#[serde(alias = "connectionlimit")]
	pub connection_limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedisSettings {
	pub host: String,
	pub port: u16,
	pub user: Option<String>,
	pub password: Option<String>,
	pub database: Option<u8>,
	#[serde(alias = "connectionlimit")]
	pub connection_limit: u32,
	pub secure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwilioSettings {
	pub username: String,
	#[serde(alias = "accesstoken")]
	pub access_token: String,
	#[serde(alias = "fromnumber")]
	pub from_number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailSettings {
	pub host: String,
	pub port: u16,
	pub secure: bool,
	pub username: String,
	pub from: String,
	pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudflareSettings {
	#[serde(alias = "accountid")]
	pub account_id: String,
	#[serde(alias = "accountemail")]
	pub account_email: String,
	#[serde(alias = "apitoken")]
	pub api_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunningEnvironment {
	Development,
	Production,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerRegistrySettings {
	#[serde(alias = "servicename")]
	pub service_name: String,
	pub issuer: String,
	#[serde(alias = "registryurl")]
	pub registry_url: String,
	#[serde(alias = "privatekey")]
	pub private_key: String,
	#[serde(alias = "publickey")]
	pub public_key: String,
	#[serde(deserialize_with = "base64_to_byte_array", alias = "publickeyder")]
	pub public_key_der: Vec<u8>,
	#[serde(alias = "authorizationheader")]
	pub authorization_header: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Digitalocean {
	#[serde(alias = "apikey")]
	pub api_key: String,
	pub registry: String,
}

impl Display for RunningEnvironment {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
		write!(
			formatter,
			"{}",
			match self {
				RunningEnvironment::Development => "Development",
				RunningEnvironment::Production => "Production",
			}
		)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KubernetesSettings {
	#[serde(alias = "certificateauthoritydata")]
	pub certificate_authority_data: String,
	#[serde(alias = "clustername")]
	pub cluster_name: String,
	#[serde(alias = "clusterurl")]
	pub cluster_url: String,
	#[serde(alias = "authname")]
	pub auth_name: String,
	#[serde(alias = "authusername")]
	pub auth_username: String,
	#[serde(alias = "authtoken")]
	pub auth_token: String,
	#[serde(alias = "contextname")]
	pub context_name: String,
	#[serde(alias = "certissuerhttp")]
	pub cert_issuer_http: String,
	#[serde(alias = "certissuerdns")]
	pub cert_issuer_dns: String,
	#[serde(alias = "staticsiteproxyservice")]
	pub static_site_proxy_service: String,
	#[serde(alias = "alertwebhooksecret")]
	pub alert_webhook_secret: String,
}

fn base64_to_byte_array<'de, D>(value: D) -> Result<Vec<u8>, D::Error>
where
	D: Deserializer<'de>,
{
	let string = String::deserialize(value)?;
	Ok(base64::decode(&string)
		.unwrap_or_else(|_| panic!("Unable to decode {} as base64", string)))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrometheusSettings {
	pub host: String,
	pub username: String,
	pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RabbitMqSettings {
	pub host: String,
	pub port: u16,
	pub queue: String,
	pub username: String,
	pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultSettings {
	pub address: String,
	pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LokiSettings {
	pub host: String,
	pub username: String,
	pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailchimpSettings {
	#[serde(alias = "apikey")]
	pub api_key: String,
	#[serde(alias = "listid")]
	pub list_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubSettings {
	#[serde(alias = "clientid")]
	pub client_id: String,
	#[serde(alias = "clientsecret")]
	pub client_secret: String,
	#[serde(alias = "authurl")]
	pub auth_url: String,
	#[serde(alias = "callbackurl")]
	pub callback_url: String,
	#[serde(alias = "useremailapi")]
	pub user_email_api: String,
	pub state: String,
	pub scope: String,
	#[serde(alias = "userInfoUrl")]
	pub user_info_url: String,
	pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleSettings {
	#[serde(alias = "clientid")]
	pub client_id: String,
	#[serde(alias = "clientsecret")]
	pub client_secret: String,
	#[serde(alias = "authurl")]
	pub auth_url: String,
	#[serde(alias = "scope")]
	pub scope: String,
	#[serde(alias = "oauthcallbackurl")]
	pub oauth_callback_url: String,
	#[serde(alias = "redirecturl")]
	pub redirect_url: String,
	#[serde(alias = "expiresin")]
	pub expires_in: u64,

}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StripeSettings {
	#[serde(alias = "secretkey")]
	pub secret_key: String,
	#[serde(alias = "publishablekey")]
	pub publishable_key: String,
}
