use api_models::{
	models::workspace::billing::{Address, StripeCustomer},
	utils::Uuid,
};
use eve_rs::AsError;
use reqwest::Client;

use crate::{
	db::{self, PaymentType},
	error,
	models::{billing::StripeAddress, rbac},
	utils::{
		constants::default_limits,
		get_current_time_millis,
		settings::Settings,
		validator,
		Error,
	},
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

	let limit = db::get_user_by_user_id(connection, super_admin_id)
		.await?
		.status(500)?
		.workspace_limit as usize;

	let super_admin_workspaces =
		db::get_all_workspaces_owned_by_user(connection, super_admin_id)
			.await?
			.len();

	if super_admin_workspaces + 1 > limit {
		return Err(Error::empty()
			.status(400)
			.body(error!(RESOURCE_LIMIT_EXCEEDED).to_string()));
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
		default_limits::DEPLOYMENTS,
		default_limits::MANAGED_DATABASE,
		default_limits::STATIC_SITES,
		default_limits::MANAGED_URLS,
		default_limits::DOCKER_REPOSITORY_STORAGE,
		default_limits::DOMAINS,
		default_limits::SECRETS,
		&stripe_customer.id,
		&PaymentType::Card,
	)
	.await?;
	db::end_deferred_constraints(connection).await?;

	super::create_kubernetes_namespace(
		resource_id.as_str(),
		config,
		&Uuid::new_v4(),
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

pub async fn add_billing_address(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	address_details: Address,
	config: &Settings,
) -> Result<(), Error> {
	if address_details.address_line_2.is_none() &&
		address_details.address_line_3.is_some()
	{
		return Error::as_result()
			.status(400)
			.body(error!(ADDRESS_LINE_3_NOT_ALLOWED).to_string())?;
	}

	let workspace = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)?;

	if workspace.address_id.is_some() {
		return Error::as_result()
			.status(400)
			.body(error!(ADDRESS_ALREADY_EXISTS).to_string())?;
	}
	let address_id = db::generate_new_address_id(connection).await?;
	let address_details = db::Address {
		id: address_id.clone(),
		first_name: address_details.first_name,
		last_name: address_details.last_name,
		address_line_1: address_details.address_line_1,
		address_line_2: address_details.address_line_2,
		address_line_3: address_details.address_line_3,
		city: address_details.city,
		state: address_details.state,
		zip: address_details.zip,
		country: address_details.country,
	};

	db::add_billing_address(connection, &address_details).await?;
	db::add_billing_address_to_workspace(connection, workspace_id, &address_id)
		.await?;

	let client = Client::new();

	let password: Option<String> = None;

	let address_line_2 = if let (Some(line2), Some(line3)) = (
		address_details.address_line_2.clone(),
		address_details.address_line_3,
	) {
		Some(format!("{} {}", line2, line3))
	} else {
		address_details.address_line_2
	};

	client
		.post(format!(
			"https://api.stripe.com/v1/customers/{}",
			workspace.stripe_customer_id
		))
		.basic_auth(config.stripe.secret_key.as_str(), password.as_ref())
		.form(&StripeAddress {
			city: address_details.city,
			country: address_details.country,
			line1: address_details.address_line_1,
			line2: address_line_2,
			postal_code: address_details.zip,
			state: address_details.state,
		})
		.send()
		.await?
		.error_for_status()?;

	Ok(())
}

pub async fn update_billing_address(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	address_details: Address,
) -> Result<(), Error> {
	if address_details.address_line_2.is_none() &&
		address_details.address_line_3.is_some()
	{
		return Error::as_result()
			.status(400)
			.body(error!(ADDRESS_LINE_3_NOT_ALLOWED).to_string())?;
	}
	let workspace_data = db::get_workspace_info(connection, workspace_id)
		.await?
		.status(500)?;
	if let Some(address_id) = workspace_data.address_id {
		let address_details = &db::Address {
			id: address_id,
			first_name: address_details.first_name,
			last_name: address_details.last_name,
			address_line_1: address_details.address_line_1,
			address_line_2: address_details.address_line_2,
			address_line_3: address_details.address_line_3,
			city: address_details.city,
			state: address_details.state,
			zip: address_details.zip,
			country: address_details.country,
		};
		db::update_billing_address(connection, address_details).await?;
	} else {
		return Error::as_result()
			.status(400)
			.body(error!(ADDRESS_NOT_FOUND).to_string())?;
	}
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
