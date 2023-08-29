//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "deployment_status")]
pub enum DeploymentStatus {
	#[sea_orm(string_value = "created")]
	Created,
	#[sea_orm(string_value = "deleted")]
	Deleted,
	#[sea_orm(string_value = "deploying")]
	Deploying,
	#[sea_orm(string_value = "errored")]
	Errored,
	#[sea_orm(string_value = "pushed")]
	Pushed,
	#[sea_orm(string_value = "running")]
	Running,
	#[sea_orm(string_value = "stopped")]
	Stopped,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "dns_record_type")]
pub enum DnsRecordType {
	#[sea_orm(string_value = "A")]
	A,
	#[sea_orm(string_value = "AAAA")]
	Aaaa,
	#[sea_orm(string_value = "CNAME")]
	Cname,
	#[sea_orm(string_value = "MX")]
	Mx,
	#[sea_orm(string_value = "TXT")]
	Txt,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(
	rs_type = "String",
	db_type = "Enum",
	enum_name = "domain_nameserver_type"
)]
pub enum DomainNameserverType {
	#[sea_orm(string_value = "external")]
	External,
	#[sea_orm(string_value = "internal")]
	Internal,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "domain_plan")]
pub enum DomainPlan {
	#[sea_orm(string_value = "free")]
	Free,
	#[sea_orm(string_value = "unlimited")]
	Unlimited,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "exposed_port_type")]
pub enum ExposedPortType {
	#[sea_orm(string_value = "http")]
	Http,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(
	rs_type = "String",
	db_type = "Enum",
	enum_name = "infrastructure_cloud_provider"
)]
pub enum InfrastructureCloudProvider {
	#[sea_orm(string_value = "digitalocean")]
	Digitalocean,
	#[sea_orm(string_value = "other")]
	Other,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(
	rs_type = "String",
	db_type = "Enum",
	enum_name = "legacy_managed_database_plan"
)]
pub enum LegacyManagedDatabasePlan {
	#[sea_orm(string_value = "large")]
	Large,
	#[sea_orm(string_value = "mammoth")]
	Mammoth,
	#[sea_orm(string_value = "medium")]
	Medium,
	#[sea_orm(string_value = "micro")]
	Micro,
	#[sea_orm(string_value = "nano")]
	Nano,
	#[sea_orm(string_value = "small")]
	Small,
	#[sea_orm(string_value = "xlarge")]
	Xlarge,
	#[sea_orm(string_value = "xxlarge")]
	Xxlarge,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(
	rs_type = "String",
	db_type = "Enum",
	enum_name = "managed_database_engine"
)]
pub enum ManagedDatabaseEngine {
	#[sea_orm(string_value = "mongo")]
	Mongo,
	#[sea_orm(string_value = "mysql")]
	Mysql,
	#[sea_orm(string_value = "postgres")]
	Postgres,
	#[sea_orm(string_value = "redis")]
	Redis,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(
	rs_type = "String",
	db_type = "Enum",
	enum_name = "managed_database_status"
)]
pub enum ManagedDatabaseStatus {
	#[sea_orm(string_value = "creating")]
	Creating,
	#[sea_orm(string_value = "deleted")]
	Deleted,
	#[sea_orm(string_value = "errored")]
	Errored,
	#[sea_orm(string_value = "running")]
	Running,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "managed_url_type")]
pub enum ManagedUrlType {
	#[sea_orm(string_value = "proxy_to_deployment")]
	ProxyToDeployment,
	#[sea_orm(string_value = "proxy_to_static_site")]
	ProxyToStaticSite,
	#[sea_orm(string_value = "proxy_url")]
	ProxyUrl,
	#[sea_orm(string_value = "redirect")]
	Redirect,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "payment_status")]
pub enum PaymentStatus {
	#[sea_orm(string_value = "failed")]
	Failed,
	#[sea_orm(string_value = "pending")]
	Pending,
	#[sea_orm(string_value = "success")]
	Success,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "payment_type")]
pub enum PaymentType {
	#[sea_orm(string_value = "card")]
	Card,
	#[sea_orm(string_value = "enterprise")]
	Enterprise,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "permission_type")]
pub enum PermissionType {
	#[sea_orm(string_value = "exclude")]
	Exclude,
	#[sea_orm(string_value = "include")]
	Include,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "region_status")]
pub enum RegionStatus {
	#[sea_orm(string_value = "active")]
	Active,
	#[sea_orm(string_value = "coming_soon")]
	ComingSoon,
	#[sea_orm(string_value = "creating")]
	Creating,
	#[sea_orm(string_value = "deleted")]
	Deleted,
	#[sea_orm(string_value = "disconnected")]
	Disconnected,
	#[sea_orm(string_value = "errored")]
	Errored,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(
	rs_type = "String",
	db_type = "Enum",
	enum_name = "resource_owner_type"
)]
pub enum ResourceOwnerType {
	#[sea_orm(string_value = "business")]
	Business,
	#[sea_orm(string_value = "personal")]
	Personal,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "static_site_plan")]
pub enum StaticSitePlan {
	#[sea_orm(string_value = "free")]
	Free,
	#[sea_orm(string_value = "pro")]
	Pro,
	#[sea_orm(string_value = "unlimited")]
	Unlimited,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(
	rs_type = "String",
	db_type = "Enum",
	enum_name = "token_permission_type"
)]
pub enum TokenPermissionType {
	#[sea_orm(string_value = "member")]
	Member,
	#[sea_orm(string_value = "super_admin")]
	SuperAdmin,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "transaction_type")]
pub enum TransactionType {
	#[sea_orm(string_value = "bill")]
	Bill,
	#[sea_orm(string_value = "credits")]
	Credits,
	#[sea_orm(string_value = "payment")]
	Payment,
}
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "user_login_type")]
pub enum UserLoginType {
	#[sea_orm(string_value = "api_token")]
	ApiToken,
	#[sea_orm(string_value = "web_login")]
	WebLogin,
}
