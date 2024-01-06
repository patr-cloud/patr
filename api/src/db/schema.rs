// @generated automatically by Diesel CLI.

pub mod sql_types {
	#[derive(diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "citext"))]
	pub struct Citext;

	#[derive(diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "deployment_status"))]
	pub struct DeploymentStatus;

	#[derive(diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "dns_record_type"))]
	pub struct DnsRecordType;

	#[derive(diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "domain_nameserver_type"))]
	pub struct DomainNameserverType;

	#[derive(diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "exposed_port_type"))]
	pub struct ExposedPortType;

	#[derive(diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "geometry"))]
	pub struct Geometry;

	#[derive(diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "infrastructure_cloud_provider"))]
	pub struct InfrastructureCloudProvider;

	#[derive(diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "managed_database_engine"))]
	pub struct ManagedDatabaseEngine;

	#[derive(diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "managed_database_status"))]
	pub struct ManagedDatabaseStatus;

	#[derive(diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "managed_url_type"))]
	pub struct ManagedUrlType;

	#[derive(diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "permission_type"))]
	pub struct PermissionType;

	#[derive(diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "region_status"))]
	pub struct RegionStatus;

	#[derive(diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "resource_owner_type"))]
	pub struct ResourceOwnerType;

	#[derive(diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "token_permission_type"))]
	pub struct TokenPermissionType;

	#[derive(diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "user_login_type"))]
	pub struct UserLoginType;
}

diesel::table! {
	business_email (local, domain_id) {
		user_id -> Uuid,
		#[max_length = 64]
		local -> Varchar,
		domain_id -> Uuid,
	}
}

diesel::table! {
	container_registry_manifest (manifest_digest) {
		manifest_digest -> Text,
	}
}

diesel::table! {
	container_registry_manifest_blob (manifest_digest, blob_digest) {
		manifest_digest -> Text,
		blob_digest -> Text,
		parent_blob_digest -> Nullable<Text>,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::Citext;

	container_registry_repository (id) {
		id -> Uuid,
		workspace_id -> Uuid,
		name -> Citext,
		deleted -> Nullable<Timestamptz>,
	}
}

diesel::table! {
	container_registry_repository_blob (blob_digest) {
		blob_digest -> Text,
		created -> Timestamptz,
		size -> Int8,
	}
}

diesel::table! {
	container_registry_repository_manifest (repository_id, manifest_digest) {
		repository_id -> Uuid,
		manifest_digest -> Text,
		created -> Timestamptz,
	}
}

diesel::table! {
	container_registry_repository_tag (repository_id, tag) {
		repository_id -> Uuid,
		tag -> Text,
		manifest_digest -> Text,
		last_updated -> Timestamptz,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::Citext;
	use super::sql_types::DeploymentStatus;
	use super::sql_types::ExposedPortType;

	deployment (id) {
		id -> Uuid,
		name -> Citext,
		#[max_length = 255]
		registry -> Varchar,
		repository_id -> Nullable<Uuid>,
		#[max_length = 512]
		image_name -> Nullable<Varchar>,
		#[max_length = 255]
		image_tag -> Varchar,
		status -> DeploymentStatus,
		workspace_id -> Uuid,
		region -> Uuid,
		min_horizontal_scale -> Int2,
		max_horizontal_scale -> Int2,
		machine_type -> Uuid,
		deploy_on_push -> Bool,
		startup_probe_port -> Nullable<Int4>,
		#[max_length = 255]
		startup_probe_path -> Nullable<Varchar>,
		startup_probe_port_type -> Nullable<ExposedPortType>,
		liveness_probe_port -> Nullable<Int4>,
		#[max_length = 255]
		liveness_probe_path -> Nullable<Varchar>,
		liveness_probe_port_type -> Nullable<ExposedPortType>,
		current_live_digest -> Nullable<Text>,
		deleted -> Nullable<Timestamptz>,
	}
}

diesel::table! {
	deployment_config_mounts (deployment_id, path) {
		path -> Text,
		file -> Bytea,
		deployment_id -> Uuid,
	}
}

diesel::table! {
	deployment_deploy_history (deployment_id, image_digest) {
		deployment_id -> Uuid,
		image_digest -> Text,
		repository_id -> Uuid,
		created -> Timestamptz,
	}
}

diesel::table! {
	deployment_environment_variable (deployment_id, name) {
		deployment_id -> Uuid,
		#[max_length = 256]
		name -> Varchar,
		value -> Nullable<Text>,
		secret_id -> Nullable<Uuid>,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::ExposedPortType;

	deployment_exposed_port (deployment_id, port) {
		deployment_id -> Uuid,
		port -> Int4,
		port_type -> ExposedPortType,
	}
}

diesel::table! {
	deployment_machine_type (id) {
		id -> Uuid,
		cpu_count -> Int2,
		memory_count -> Int4,
	}
}

diesel::table! {
	deployment_volume (id) {
		id -> Uuid,
		name -> Text,
		deployment_id -> Uuid,
		volume_size -> Int4,
		volume_mount_path -> Text,
		deleted -> Nullable<Timestamptz>,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::ResourceOwnerType;

	domain (id) {
		id -> Uuid,
		name -> Text,
		#[sql_name = "type"]
		type_ -> ResourceOwnerType,
		tld -> Text,
		deleted -> Nullable<Timestamptz>,
	}
}

diesel::table! {
	domain_tld (tld) {
		tld -> Text,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::Citext;
	use super::sql_types::ManagedDatabaseEngine;
	use super::sql_types::ManagedDatabaseStatus;

	managed_database (id) {
		id -> Uuid,
		name -> Citext,
		workspace_id -> Uuid,
		region -> Uuid,
		engine -> ManagedDatabaseEngine,
		database_plan_id -> Uuid,
		status -> ManagedDatabaseStatus,
		username -> Text,
		deleted -> Nullable<Timestamptz>,
	}
}

diesel::table! {
	managed_database_plan (id) {
		id -> Uuid,
		cpu -> Int4,
		ram -> Int4,
		volume -> Int4,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::ManagedUrlType;

	managed_url (id) {
		id -> Uuid,
		sub_domain -> Text,
		domain_id -> Uuid,
		path -> Text,
		url_type -> ManagedUrlType,
		deployment_id -> Nullable<Uuid>,
		port -> Nullable<Int4>,
		static_site_id -> Nullable<Uuid>,
		url -> Nullable<Text>,
		workspace_id -> Uuid,
		is_configured -> Bool,
		deleted -> Nullable<Timestamptz>,
		permanent_redirect -> Nullable<Bool>,
		http_only -> Nullable<Bool>,
		cloudflare_custom_hostname_id -> Text,
	}
}

diesel::table! {
	meta_data (id) {
		id -> Text,
		value -> Text,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::DomainNameserverType;

	patr_controlled_domain (domain_id) {
		domain_id -> Uuid,
		zone_identifier -> Text,
		nameserver_type -> DomainNameserverType,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::DnsRecordType;

	patr_domain_dns_record (id) {
		id -> Uuid,
		record_identifier -> Text,
		domain_id -> Uuid,
		name -> Text,
		#[sql_name = "type"]
		type_ -> DnsRecordType,
		value -> Text,
		priority -> Nullable<Int4>,
		ttl -> Int8,
		proxied -> Nullable<Bool>,
	}
}

diesel::table! {
	permission (id) {
		id -> Uuid,
		#[max_length = 100]
		name -> Varchar,
		#[max_length = 500]
		description -> Varchar,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::ResourceOwnerType;

	personal_domain (id) {
		id -> Uuid,
		domain_type -> ResourceOwnerType,
		deleted -> Nullable<Timestamptz>,
	}
}

diesel::table! {
	personal_email (local, domain_id) {
		user_id -> Nullable<Uuid>,
		#[max_length = 64]
		local -> Varchar,
		domain_id -> Uuid,
	}
}

diesel::table! {
	phone_number_country_code (country_code) {
		#[max_length = 2]
		country_code -> Bpchar,
		#[max_length = 5]
		phone_code -> Varchar,
		#[max_length = 80]
		country_name -> Varchar,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::InfrastructureCloudProvider;
	use super::sql_types::RegionStatus;

	region (id) {
		id -> Uuid,
		name -> Text,
		provider -> InfrastructureCloudProvider,
		workspace_id -> Nullable<Uuid>,
		message_log -> Nullable<Text>,
		status -> RegionStatus,
		ingress_hostname -> Nullable<Text>,
		cloudflare_certificate_id -> Nullable<Text>,
		config_file -> Nullable<Json>,
		deleted -> Nullable<Timestamptz>,
		disconnected_at -> Nullable<Timestamptz>,
	}
}

diesel::table! {
	resource (id) {
		id -> Uuid,
		resource_type_id -> Nullable<Uuid>,
		owner_id -> Uuid,
		created -> Timestamptz,
	}
}

diesel::table! {
	resource_type (id) {
		id -> Uuid,
		#[max_length = 100]
		name -> Varchar,
		#[max_length = 500]
		description -> Varchar,
	}
}

diesel::table! {
	role (id) {
		id -> Uuid,
		#[max_length = 100]
		name -> Varchar,
		#[max_length = 500]
		description -> Varchar,
		owner_id -> Uuid,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::PermissionType;

	role_resource_permissions_exclude (role_id, permission_id, resource_id) {
		role_id -> Uuid,
		permission_id -> Uuid,
		resource_id -> Uuid,
		permission_type -> PermissionType,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::PermissionType;

	role_resource_permissions_include (role_id, permission_id, resource_id) {
		role_id -> Uuid,
		permission_id -> Uuid,
		resource_id -> Uuid,
		permission_type -> PermissionType,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::PermissionType;

	role_resource_permissions_type (role_id, permission_id) {
		role_id -> Uuid,
		permission_id -> Uuid,
		permission_type -> PermissionType,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::Citext;

	secret (id) {
		id -> Uuid,
		name -> Citext,
		workspace_id -> Uuid,
		deleted -> Nullable<Timestamptz>,
	}
}

diesel::table! {
	spatial_ref_sys (srid) {
		srid -> Int4,
		#[max_length = 256]
		auth_name -> Nullable<Varchar>,
		auth_srid -> Nullable<Int4>,
		#[max_length = 2048]
		srtext -> Nullable<Varchar>,
		#[max_length = 2048]
		proj4text -> Nullable<Varchar>,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::Citext;
	use super::sql_types::DeploymentStatus;

	static_site (id) {
		id -> Uuid,
		name -> Citext,
		status -> DeploymentStatus,
		workspace_id -> Uuid,
		current_live_upload -> Nullable<Uuid>,
		deleted -> Nullable<Timestamptz>,
	}
}

diesel::table! {
	static_site_upload_history (upload_id) {
		upload_id -> Uuid,
		static_site_id -> Uuid,
		message -> Text,
		uploaded_by -> Uuid,
		created -> Timestamptz,
		processed -> Nullable<Timestamptz>,
	}
}

diesel::table! {
	user (id) {
		id -> Uuid,
		#[max_length = 100]
		username -> Varchar,
		password -> Text,
		#[max_length = 100]
		first_name -> Varchar,
		#[max_length = 100]
		last_name -> Varchar,
		created -> Timestamptz,
		#[max_length = 64]
		recovery_email_local -> Nullable<Varchar>,
		recovery_email_domain_id -> Nullable<Uuid>,
		#[max_length = 2]
		recovery_phone_country_code -> Nullable<Bpchar>,
		#[max_length = 15]
		recovery_phone_number -> Nullable<Varchar>,
		workspace_limit -> Int4,
		password_reset_token -> Nullable<Text>,
		password_reset_token_expiry -> Timestamptz,
		password_reset_attempts -> Int4,
		mfa_secret -> Nullable<Text>,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::UserLoginType;

	user_api_token (token_id) {
		token_id -> Uuid,
		name -> Text,
		user_id -> Uuid,
		token_hash -> Text,
		token_nbf -> Nullable<Timestamptz>,
		token_exp -> Nullable<Timestamptz>,
		allowed_ips -> Nullable<Array<Nullable<Inet>>>,
		created -> Timestamptz,
		revoked -> Nullable<Timestamptz>,
		login_type -> Nullable<UserLoginType>,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::PermissionType;

	user_api_token_resource_permissions_exclude (token_id, workspace_id, permission_id, resource_id) {
		token_id -> Uuid,
		workspace_id -> Uuid,
		permission_id -> Uuid,
		resource_id -> Uuid,
		permission_type -> PermissionType,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::PermissionType;

	user_api_token_resource_permissions_include (token_id, workspace_id, permission_id, resource_id) {
		token_id -> Uuid,
		workspace_id -> Uuid,
		permission_id -> Uuid,
		resource_id -> Uuid,
		permission_type -> PermissionType,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::PermissionType;
	use super::sql_types::TokenPermissionType;

	user_api_token_resource_permissions_type (token_id, workspace_id, permission_id) {
		token_id -> Uuid,
		workspace_id -> Uuid,
		permission_id -> Uuid,
		resource_permission_type -> PermissionType,
		token_permission_type -> TokenPermissionType,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::TokenPermissionType;

	user_api_token_workspace_permission_type (token_id, workspace_id) {
		token_id -> Uuid,
		workspace_id -> Uuid,
		token_permission_type -> TokenPermissionType,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::TokenPermissionType;

	user_api_token_workspace_super_admin (token_id, user_id, workspace_id) {
		token_id -> Uuid,
		user_id -> Uuid,
		workspace_id -> Uuid,
		token_permission_type -> TokenPermissionType,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::DomainNameserverType;

	user_controlled_domain (domain_id) {
		domain_id -> Uuid,
		nameserver_type -> DomainNameserverType,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::UserLoginType;

	user_login (login_id) {
		login_id -> Uuid,
		user_id -> Uuid,
		login_type -> UserLoginType,
		created -> Timestamptz,
	}
}

diesel::table! {
	user_phone_number (country_code, number) {
		user_id -> Uuid,
		#[max_length = 2]
		country_code -> Bpchar,
		#[max_length = 15]
		number -> Varchar,
	}
}

diesel::table! {
	user_to_sign_up (username) {
		#[max_length = 100]
		username -> Varchar,
		password -> Text,
		#[max_length = 100]
		first_name -> Varchar,
		#[max_length = 100]
		last_name -> Varchar,
		#[max_length = 64]
		recovery_email_local -> Nullable<Varchar>,
		recovery_email_domain_id -> Nullable<Uuid>,
		#[max_length = 2]
		recovery_phone_country_code -> Nullable<Bpchar>,
		#[max_length = 15]
		recovery_phone_number -> Nullable<Varchar>,
		otp_hash -> Text,
		otp_expiry -> Timestamptz,
	}
}

diesel::table! {
	user_unverified_personal_email (local, domain_id) {
		#[max_length = 64]
		local -> Varchar,
		domain_id -> Uuid,
		user_id -> Uuid,
		verification_token_hash -> Text,
		verification_token_expiry -> Timestamptz,
	}
}

diesel::table! {
	user_unverified_phone_number (country_code, phone_number) {
		#[max_length = 2]
		country_code -> Bpchar,
		#[max_length = 15]
		phone_number -> Varchar,
		user_id -> Uuid,
		verification_token_hash -> Text,
		verification_token_expiry -> Timestamptz,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::Geometry;
	use super::sql_types::UserLoginType;

	web_login (login_id) {
		login_id -> Uuid,
		original_login_id -> Nullable<Uuid>,
		user_id -> Uuid,
		refresh_token -> Text,
		token_expiry -> Timestamptz,
		created -> Timestamptz,
		created_ip -> Inet,
		created_location -> Geometry,
		created_user_agent -> Text,
		created_country -> Text,
		created_region -> Text,
		created_city -> Text,
		created_timezone -> Text,
		login_type -> UserLoginType,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::Citext;

	workspace (id) {
		id -> Uuid,
		name -> Citext,
		super_admin_id -> Uuid,
		deleted -> Nullable<Timestamptz>,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::ResourceOwnerType;
	use super::sql_types::DomainNameserverType;

	workspace_domain (id) {
		id -> Uuid,
		domain_type -> ResourceOwnerType,
		is_verified -> Bool,
		nameserver_type -> DomainNameserverType,
		last_unverified -> Nullable<Timestamptz>,
		cloudflare_worker_route_id -> Text,
	}
}

diesel::table! {
	workspace_user (user_id, workspace_id, role_id) {
		user_id -> Uuid,
		workspace_id -> Uuid,
		role_id -> Uuid,
	}
}

diesel::joinable!(business_email -> user (user_id));
diesel::joinable!(business_email -> workspace_domain (domain_id));
diesel::joinable!(container_registry_manifest_blob -> container_registry_manifest (manifest_digest));
diesel::joinable!(container_registry_repository -> workspace (workspace_id));
diesel::joinable!(container_registry_repository_manifest -> container_registry_manifest (manifest_digest));
diesel::joinable!(container_registry_repository_manifest -> container_registry_repository (repository_id));
diesel::joinable!(container_registry_repository_tag -> container_registry_repository (repository_id));
diesel::joinable!(deployment -> deployment_machine_type (machine_type));
diesel::joinable!(deployment -> region (region));
diesel::joinable!(deployment_config_mounts -> deployment (deployment_id));
diesel::joinable!(deployment_deploy_history -> container_registry_repository (repository_id));
diesel::joinable!(deployment_environment_variable -> deployment (deployment_id));
diesel::joinable!(deployment_environment_variable -> secret (secret_id));
diesel::joinable!(deployment_volume -> deployment (deployment_id));
diesel::joinable!(domain -> domain_tld (tld));
diesel::joinable!(managed_database -> managed_database_plan (database_plan_id));
diesel::joinable!(managed_database -> region (region));
diesel::joinable!(managed_database -> workspace (workspace_id));
diesel::joinable!(managed_url -> workspace_domain (domain_id));
diesel::joinable!(patr_domain_dns_record -> patr_controlled_domain (domain_id));
diesel::joinable!(patr_domain_dns_record -> resource (id));
diesel::joinable!(personal_email -> personal_domain (domain_id));
diesel::joinable!(region -> workspace (workspace_id));
diesel::joinable!(resource -> resource_type (resource_type_id));
diesel::joinable!(role -> workspace (owner_id));
diesel::joinable!(role_resource_permissions_exclude -> resource (resource_id));
diesel::joinable!(role_resource_permissions_include -> resource (resource_id));
diesel::joinable!(role_resource_permissions_type -> permission (permission_id));
diesel::joinable!(role_resource_permissions_type -> role (role_id));
diesel::joinable!(static_site_upload_history -> resource (upload_id));
diesel::joinable!(static_site_upload_history -> user (uploaded_by));
diesel::joinable!(user_api_token_resource_permissions_type -> permission (permission_id));
diesel::joinable!(user_api_token_workspace_permission_type -> user_api_token (token_id));
diesel::joinable!(user_login -> user (user_id));
diesel::joinable!(user_phone_number -> phone_number_country_code (country_code));
diesel::joinable!(user_to_sign_up -> personal_domain (recovery_email_domain_id));
diesel::joinable!(user_to_sign_up -> phone_number_country_code (recovery_phone_country_code));
diesel::joinable!(user_unverified_personal_email -> personal_domain (domain_id));
diesel::joinable!(user_unverified_personal_email -> user (user_id));
diesel::joinable!(user_unverified_phone_number -> phone_number_country_code (country_code));
diesel::joinable!(user_unverified_phone_number -> user (user_id));
diesel::joinable!(workspace -> user (super_admin_id));
diesel::joinable!(workspace_domain -> resource (id));
diesel::joinable!(workspace_user -> role (role_id));
diesel::joinable!(workspace_user -> user (user_id));
diesel::joinable!(workspace_user -> workspace (workspace_id));

diesel::allow_tables_to_appear_in_same_query!(
	business_email,
	container_registry_manifest,
	container_registry_manifest_blob,
	container_registry_repository,
	container_registry_repository_blob,
	container_registry_repository_manifest,
	container_registry_repository_tag,
	deployment,
	deployment_config_mounts,
	deployment_deploy_history,
	deployment_environment_variable,
	deployment_exposed_port,
	deployment_machine_type,
	deployment_volume,
	domain,
	domain_tld,
	managed_database,
	managed_database_plan,
	managed_url,
	meta_data,
	patr_controlled_domain,
	patr_domain_dns_record,
	permission,
	personal_domain,
	personal_email,
	phone_number_country_code,
	region,
	resource,
	resource_type,
	role,
	role_resource_permissions_exclude,
	role_resource_permissions_include,
	role_resource_permissions_type,
	secret,
	spatial_ref_sys,
	static_site,
	static_site_upload_history,
	user,
	user_api_token,
	user_api_token_resource_permissions_exclude,
	user_api_token_resource_permissions_include,
	user_api_token_resource_permissions_type,
	user_api_token_workspace_permission_type,
	user_api_token_workspace_super_admin,
	user_controlled_domain,
	user_login,
	user_phone_number,
	user_to_sign_up,
	user_unverified_personal_email,
	user_unverified_phone_number,
	web_login,
	workspace,
	workspace_domain,
	workspace_user,
);
