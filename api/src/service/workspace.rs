use api_models::utils::Uuid;
use eve_rs::AsError;
use reqwest::Client;

use crate::{
	db,
	error,
	models::{rbac, StripeCustomer},
	utils::{get_current_time_millis, settings::Settings, validator, Error},
	Database,
};

/// # Description
/// This function is used to check if the workspace name is valid
/// or if it is already present in the database
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `workspace_name` - a string containing name of the workspace
///
/// # Returns
/// This function returns Result<bool, Error> containing bool which either
/// contains a boolean stating whether the workspace name is allowed or not
/// or an error
///
/// [`Transaction`]: Transaction
pub async fn is_workspace_name_allowed(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_name: &str,
	allow_personal_workspaces: bool,
) -> Result<bool, Error> {
	// If personal workspaces are not allowed and the validator check fails,
	// then throw an error
	if !allow_personal_workspaces &&
		!validator::is_workspace_name_valid(workspace_name)
	{
		Error::as_result()
			.status(200)
			.body(error!(INVALID_WORKSPACE_NAME).to_string())?;
	}

	let workspace =
		db::get_workspace_by_name(connection, workspace_name).await?;
	if workspace.is_some() {
		return Ok(false);
	}

	let workspace_sign_up_status =
		db::get_user_to_sign_up_by_business_name(connection, workspace_name)
			.await?;

	if let Some(status) = workspace_sign_up_status {
		if status.otp_expiry > get_current_time_millis() {
			return Ok(false);
		}
	}
	Ok(true)
}

/// # Description
/// This function is used to create workspace
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `workspace_name` - a string containing name of the workspace
/// * `super_admin_id` - an unsigned 8 bit integer array containing id of the
/// super admin of
/// workspace
///
/// # Returns
/// This function returns `Result<Uuid, Error>` containing workspace id
/// (uuid) or an error
///
/// [`Transaction`]: Transaction
pub async fn create_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_name: &str,
	super_admin_id: &Uuid,
	allow_personal_workspaces: bool,
	alert_emails: &[String],
	config: &Settings,
) -> Result<Uuid, Error> {
	if !is_workspace_name_allowed(
		connection,
		workspace_name,
		allow_personal_workspaces,
	)
	.await?
	{
		Error::as_result()
			.status(400)
			.body(error!(WORKSPACE_EXISTS).to_string())?;
	}

	let resource_id = db::generate_new_resource_id(connection).await?;

	db::begin_deferred_constraints(connection).await?;
	db::create_resource(
		connection,
		&resource_id,
		&format!("Workspace: {}", workspace_name),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::WORKSPACE)
			.unwrap(),
		&resource_id,
		get_current_time_millis(),
	)
	.await?;

	let stripe_customer = create_stripe_customer(&resource_id, config).await?;

	db::create_workspace(
		connection,
		&resource_id,
		workspace_name,
		super_admin_id,
		alert_emails,
		&stripe_customer.id,
	)
	.await?;
	db::end_deferred_constraints(connection).await?;

	super::create_kubernetes_namespace(
		resource_id.as_str(),
		config,
		&Uuid::new_v4(),
	)
	.await?;

	let user = db::get_user_by_user_id(connection, super_admin_id)
		.await?
		.status(500)?;

	super::create_chargebee_user(
		&resource_id,
		&user.first_name,
		&user.last_name,
		config,
	)
	.await?;

	// creating initial resourc limits for the user
	create_resource_and_product_limits_for_workspace(
		connection,
		&resource_id,
		&super_admin_id,
		config,
	)
	.await?;

	Ok(resource_id)
}

/// # Description
/// This function is used to convert username into personal workspace name
///
/// # Arguments
/// * `username` - a string containing username of the user
///
/// # Returns
/// This function returns a string containing the name of the personal
/// workspace
pub fn get_personal_workspace_name(super_admin_id: &Uuid) -> String {
	format!("personal-workspace-{}", super_admin_id)
}

async fn create_resource_and_product_limits_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	super_admin_id: &Uuid,
	config: &Settings,
) -> Result<(), Error> {
	// temporarily hardcoding the limits for the workspace
	let resource_limit_id =
		db::generate_new_resource_limit_id(connection).await?;
	db::create_resource_limit(
		connection,
		&resource_limit_id,
		&workspace_id,
		20 as u32,
	)
	.await?;

	let deployment_product_id =
		db::get_product_info_by_name(connection, "deployment")
			.await?
			.status(500)?;

	db::create_product_limit(
		connection,
		&workspace_id,
		&deployment_product_id.id,
		5,
	)
	.await?;

	let static_site_product_id =
		db::get_product_info_by_name(connection, "static-site")
			.await?
			.status(500)?;

	db::create_product_limit(
		connection,
		&workspace_id,
		&static_site_product_id.id,
		5,
	)
	.await?;

	let managed_database_product_id =
		db::get_product_info_by_name(connection, "managed-database")
			.await?
			.status(500)?;

	db::create_product_limit(
		connection,
		&workspace_id,
		&managed_database_product_id.id,
		5,
	)
	.await?;

	let managed_url_product_id =
		db::get_product_info_by_name(connection, "managed-url")
			.await?
			.status(500)?;

	db::create_product_limit(
		connection,
		&workspace_id,
		&managed_url_product_id.id,
		5,
	)
	.await?;

	Ok(())
}

async fn create_stripe_customer(
	workspace_id: &Uuid,
	config: &Settings,
) -> Result<StripeCustomer, Error> {
	let client = Client::new();

	let password: Option<String> = None;

	client
		.post("https://api.stripe.com/v1/customers")
		.basic_auth(&config.stripe.secret_key, password)
		.query(&[("name", workspace_id.as_str())])
		.send()
		.await?
		.json::<StripeCustomer>()
		.await
		.map_err(|e| e.into())
}
