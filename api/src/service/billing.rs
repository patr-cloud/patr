use api_models::utils::{DateTime, Uuid};
use chrono::Utc;
use eve_rs::AsError;
use reqwest::Client;

use crate::{
	db::{self, Coupon, PlanType, TransactionType},
	error,
	models::{deployment, ProductLimits},
	utils::{limits::free_limits, settings::Settings, Error},
	Database,
};

pub async fn create_billable_service_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	active: bool,
) -> Result<Uuid, Error> {
	log::trace!("deployment id: {}", deployment_id.as_str());

	let deployment =
		db::get_deployment_by_id_all_status(connection, deployment_id)
			.await?
			.status(500)?;

	log::trace!("TEST2");

	let plan_name = db::get_deployment_machine_type_by_id(
		connection,
		&deployment.machine_type,
	)
	.await?
	.status(500)?
	.plan_name
	.status(500)?;

	log::trace!("PLAN NAME: {}", plan_name);

	log::trace!("PLAN INFO: {}", workspace_id);

	log::trace!("TEST3");

	let plan_info = db::get_plan_by_name(connection, &plan_name, workspace_id)
		.await?
		.status(500)?;

	log::trace!("TEST4");

	let billable_service_id =
		db::generate_new_billable_service_id(connection).await?;

	log::trace!("TEST5");

	db::create_billable_service(
		connection,
		&billable_service_id,
		&plan_info.id,
		workspace_id,
		plan_info.price,
		Some(deployment.min_horizontal_scale as i32),
		&plan_info.product_info_id,
		deployment_id,
		Utc::now().into(),
		active,
	)
	.await?;

	log::trace!("billable service created");

	Ok(billable_service_id)
}

pub async fn create_billable_service_for_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	static_site_id: &Uuid,
	active: bool,
) -> Result<Uuid, Error> {
	let static_site = db::get_static_site_by_id(connection, static_site_id)
		.await?
		.status(500)?;

	let static_site_product_id =
		db::get_product_info_by_name(connection, "static-site")
			.await?
			.status(500)?
			.id;

	let active_plan = db::get_active_plan_for_product_in_workspace(
		connection,
		workspace_id,
		&static_site_product_id,
	)
	.await?
	.status(500)?;

	let plan_info = db::get_plan_by_id(connection, &active_plan.plan_id)
		.await?
		.status(500)?;

	let billable_service_id =
		db::generate_new_billable_service_id(connection).await?;

	db::create_billable_service(
		connection,
		&billable_service_id,
		&plan_info.id,
		workspace_id,
		plan_info.price,
		None,
		&plan_info.product_info_id,
		static_site_id,
		Utc::now().into(),
		active,
	)
	.await?;

	Ok(billable_service_id)
}

pub async fn upgrade_limits_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	product_limits: &ProductLimits,
) -> Result<(), Error> {
	let payment_methods =
		db::get_payment_methods_for_workspace(connection, workspace_id).await?;

	if payment_methods.len() == 1 {
		let deployment_product =
			db::get_product_info_by_name(connection, "deployment")
				.await?
				.status(500)?;
		db::update_product_limits(
			connection,
			workspace_id,
			&deployment_product.id,
			product_limits.deployment,
		)
		.await?;

		let static_site_product =
			db::get_product_info_by_name(connection, "static-site")
				.await?
				.status(500)?;
		db::update_product_limits(
			connection,
			workspace_id,
			&static_site_product.id,
			product_limits.static_site,
		)
		.await?;

		let managed_database_product =
			db::get_product_info_by_name(connection, "managed-database")
				.await?
				.status(500)?;
		db::update_product_limits(
			connection,
			workspace_id,
			&managed_database_product.id,
			product_limits.managed_database,
		)
		.await?;

		let managed_url_product =
			db::get_product_info_by_name(connection, "managed-url")
				.await?
				.status(500)?;
		db::update_product_limits(
			connection,
			workspace_id,
			&managed_url_product.id,
			product_limits.managed_url,
		)
		.await?;

		let secret_product = db::get_product_info_by_name(connection, "secret")
			.await?
			.status(500)?;
		db::update_product_limits(
			connection,
			workspace_id,
			&secret_product.id,
			product_limits.secret,
		)
		.await?;

		let domain_product = db::get_product_info_by_name(connection, "domain")
			.await?
			.status(500)?;

		db::update_product_limits(
			connection,
			workspace_id,
			&domain_product.id,
			product_limits.domain,
		)
		.await?;
	}

	Ok(())
}

pub async fn delete_payment_method(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	payment_method_id: &str,
	config: &Settings,
) -> Result<(), Error> {
	// check if there are any active resources above the free plan
	// maybe find a better way to do this by using some complex joins
	if deployment_free_limit_crossed(connection, workspace_id).await? ||
		static_site_free_limit_crossed(connection, workspace_id).await? ||
		managed_database_free_limit_crossed(connection, workspace_id).await? ||
		managed_url_free_limit_crossed(connection, workspace_id).await? ||
		secret_free_limit_crossed(connection, workspace_id).await? ||
		domain_free_limit_crossed(connection, workspace_id).await?
	{
		let payment_methods =
			db::get_payment_methods_for_workspace(connection, workspace_id)
				.await?;

		if payment_methods.len() == 1 {
			return Error::as_result()
				.status(400)
				.body(error!(CANNOT_DELETE_PAYMENT_METHOD).to_string())?;
		}
	}

	// check if the payment method is primary
	let primary_payment_method =
		db::get_workspace_info(connection, workspace_id)
			.await?
			.status(500)?
			.primary_payment_method
			.status(500)?;

	if payment_method_id == primary_payment_method {
		return Error::as_result()
			.status(400)
			.body(error!(CHANGE_PRIMARY_PAYMENT_METHOD).to_string())?;
	}

	db::delete_payment_method(connection, payment_method_id).await?;

	let client = Client::new();

	let password: Option<String> = None;

	let deletion_status = client
		.post(format!(
			"https://api.stripe.com/v1/payment_methods/{}/detach",
			payment_method_id
		))
		.basic_auth(&config.stripe.secret_key, password)
		.send()
		.await?
		.status();

	if !deletion_status.is_success() {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	}

	Ok(())
}

pub async fn initialize_plans_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<(), Error> {
	// TODO: revisit the prices of the plans
	// TODO: introduce constants here
	let deployment_product_id =
		db::get_product_info_by_name(connection, "deployment")
			.await?
			.status(500)?
			.id;

	let static_site_product_id =
		db::get_product_info_by_name(connection, "static-site")
			.await?
			.status(500)?
			.id;

	let managed_database_product =
		db::get_product_info_by_name(connection, "managed-database")
			.await?
			.status(500)?
			.id;

	let managed_url_product_id =
		db::get_product_info_by_name(connection, "managed-url")
			.await?
			.status(500)?
			.id;

	let secret_product_id = db::get_product_info_by_name(connection, "secret")
		.await?
		.status(500)?
		.id;

	let domain_product_id = db::get_product_info_by_name(connection, "domain")
		.await?
		.status(500)?
		.id;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"deployment-nano",
		None,
		&PlanType::DynamicMonthly,
		&deployment_product_id,
		0.006944,
		None,
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"deployment-micro",
		None,
		&PlanType::DynamicMonthly,
		&deployment_product_id,
		0.013888,
		None,
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"deployment-small",
		None,
		&PlanType::DynamicMonthly,
		&deployment_product_id,
		0.027777,
		None,
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"deployment-medium",
		None,
		&PlanType::DynamicMonthly,
		&deployment_product_id,
		0.055556,
		None,
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"deployment-large",
		None,
		&PlanType::DynamicMonthly,
		&deployment_product_id,
		0.111112,
		None,
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"managed-database-nano",
		None,
		&PlanType::DynamicMonthly,
		&managed_database_product,
		0.022,
		None,
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"managed-database-micro",
		None,
		&PlanType::DynamicMonthly,
		&managed_database_product,
		0.043,
		None,
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"managed-database-medium",
		None,
		&PlanType::DynamicMonthly,
		&managed_database_product,
		0.084,
		None,
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"managed-database-large",
		None,
		&PlanType::DynamicMonthly,
		&managed_database_product,
		0.1667,
		None,
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"managed-database-xlarge",
		None,
		&PlanType::DynamicMonthly,
		&managed_database_product,
		0.334,
		None,
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"managed-database-xxlarge",
		None,
		&PlanType::DynamicMonthly,
		&managed_database_product,
		0.667,
		None,
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"managed-database-mammoth",
		None,
		&PlanType::DynamicMonthly,
		&managed_database_product,
		1.334,
		None,
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"static-site-5",
		None,
		&PlanType::FixedMonthly,
		&static_site_product_id,
		0f64,
		Some(5),
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"static-site-100",
		None,
		&PlanType::FixedMonthly,
		&static_site_product_id,
		5f64,
		Some(25),
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"static-site-unlimited",
		None,
		&PlanType::FixedMonthly,
		&static_site_product_id,
		10f64,
		None,
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"managed-url-free",
		None,
		&PlanType::FixedMonthly,
		&managed_url_product_id,
		0f64,
		Some(10),
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"managed-url-100",
		None,
		&PlanType::FixedMonthly,
		&managed_url_product_id,
		10f64,
		Some(100),
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"domain-free",
		None,
		&PlanType::FixedMonthly,
		&domain_product_id,
		0f64,
		Some(1),
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"domain-unlimited",
		None,
		&PlanType::FixedMonthly,
		&domain_product_id,
		10f64,
		None,
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"secrets-free",
		None,
		&PlanType::FixedMonthly,
		&secret_product_id,
		0f64,
		Some(3),
		workspace_id,
	)
	.await?;

	let plan_id = db::generate_new_plan_id(connection).await?;
	db::create_new_plan(
		connection,
		&plan_id,
		"secrets-100",
		None,
		&PlanType::FixedMonthly,
		&secret_product_id,
		10f64,
		Some(100),
		workspace_id,
	)
	.await?;

	

	Ok(())
}

pub async fn resource_limit_crossed(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<bool, Error> {
	log::trace!("getting total resources from db");
	let total_resources =
		db::get_total_billable_resource_in_workspace(connection, workspace_id)
			.await?;

	log::trace!("getting resource limit from the db");
	let resource_limit = db::get_resource_limit(connection, workspace_id)
		.await?
		.status(500)?;

	log::trace!("total resources: {}", total_resources);
	log::trace!("resource limit: {}", resource_limit.max_resources);
	log::trace!("comparing total resources to resource limit");
	if total_resources >= resource_limit.max_resources as u64 {
		return Ok(true);
	}

	Ok(false)
}

pub async fn deployment_limit_crossed(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<bool, Error> {
	let deployments =
		db::get_deployments_for_workspace(connection, workspace_id).await?;

	log::trace!("getting product info by id");
	let deployment_product_id =
		db::get_product_info_by_name(connection, "deployment")
			.await?
			.status(500)?
			.id;

	log::trace!("getting product limits");
	let deployment_limits =
		db::get_product_limit(connection, workspace_id, &deployment_product_id)
			.await?
			.status(500)?;

	if deployments.len() == deployment_limits.max_limit as usize {
		return Ok(true);
	}

	Ok(false)
}

async fn deployment_free_limit_crossed(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<bool, Error> {
	let deployments =
		db::get_deployments_for_workspace(connection, workspace_id).await?;

	if deployments.len() == 1 {
		let (cpu_count, memory_count) = deployment::MACHINE_TYPES
			.get()
			.unwrap()
			.get(&deployments[0].machine_type)
			.unwrap_or(&(1, 2));

		if free_limits::FREE_CPU != cpu_count ||
			free_limits::FREE_MEMORY != memory_count
		{
			return Ok(true);
		}
	}

	if free_limits::FREE_DEPLOYMENTS < &(deployments.len() as u32) {
		return Ok(true);
	}

	Ok(false)
}

pub async fn add_coupon_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	coupon: &Coupon,
) -> Result<(), Error> {
	let current_coupon =
		db::get_active_coupon_by_id(connection, &coupon.id, &workspace_id)
			.await?;

	if let Some(current_coupon) = current_coupon {
		if current_coupon.remaining_usage == 0 {
			return Error::as_result()
				.status(400)
				.body(error!(COUPON_USED).to_string())?;
		}

		db::update_active_coupon_usage(
			connection,
			&coupon.id,
			&workspace_id,
			&current_coupon.remaining_usage - 1,
		)
		.await?;
	}

	db::add_coupon_to_workspace(
		connection,
		&workspace_id,
		&coupon.id,
		&coupon.num_usage,
	)
	.await?;

	let transaction_id = db::generate_new_transaction_id(connection).await?;

	db::create_transaction(
		connection,
		&transaction_id,
		None,
		&TransactionType::Onetime,
		None,
		coupon.credits,
		None,
		&workspace_id,
		&DateTime::from(Utc::now()),
		Some(&coupon.id),
		false,
	)
	.await?;

	Ok(())
}
async fn static_site_free_limit_crossed(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<bool, Error> {
	let static_sites =
		db::get_static_sites_for_workspace(connection, workspace_id).await?;

	if free_limits::FREE_STATIC_SITES < &(static_sites.len() as u32) {
		return Ok(true);
	}

	Ok(false)
}

async fn managed_database_free_limit_crossed(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<bool, Error> {
	let managed_databases =
		db::get_all_database_clusters_for_workspace(connection, workspace_id)
			.await?;

	if free_limits::FREE_MANAGED_DATABASE < &(managed_databases.len() as u32) {
		return Ok(true);
	}

	Ok(false)
}

async fn managed_url_free_limit_crossed(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<bool, Error> {
	let managed_urls =
		db::get_all_managed_urls_in_workspace(connection, workspace_id).await?;

	if free_limits::FREE_MANAGED_URLS < &(managed_urls.len() as u32) {
		return Ok(true);
	}

	Ok(false)
}

async fn secret_free_limit_crossed(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<bool, Error> {
	let secrets =
		db::get_all_secrets_in_workspace(connection, workspace_id).await?;

	if free_limits::FREE_SECRETS < &(secrets.len() as u32) {
		return Ok(true);
	}

	Ok(false)
}

async fn domain_free_limit_crossed(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<bool, Error> {
	let domains =
		db::get_domains_for_workspace(connection, workspace_id).await?;

	if free_limits::FREE_DOMAINS < &(domains.len() as u32) {
		return Ok(true);
	}

	Ok(false)
}
