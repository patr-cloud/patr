use api_models::utils::Uuid;

#[derive(sqlx::Type, Debug, PartialEq)]
#[sqlx(type_name = "MANAGED_URL_TYPE", rename_all = "lowercase")]
pub enum ManagedUrlType {
	ProxyToDeployment,
	ProxyToStaticSite,
	ProxyUrl,
	Redirect,
}

pub struct ManagedUrl {
	pub id: Uuid,
	pub sub_domain: String,
	pub domain_id: Uuid,
	pub path: String,
	pub url_type: ManagedUrlType,
	pub deployment_id: Option<Uuid>,
	pub port: Option<i32>,
	pub static_site_id: Option<Uuid>,
	pub url: Option<String>,
	pub workspace_id: Uuid,
}
