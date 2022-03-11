use api_models::{
	models::workspace::{
		billing::{
			Card,
			GetCardDetailsResponse,
			GetCreditBalanceResponse,
			GetSubscriptionsResponse,
			PaymentSource,
			PromotionalCredits,
			Subscription,
			SubscriptionItem,
			UpdateBillingInfoRequest,
			UpdateBillingInfoResponse,
		},
		CreateNewWorkspaceRequest,
		CreateNewWorkspaceResponse,
		GetWorkspaceAuditLogResponse,
		GetWorkspaceInfoResponse,
		IsWorkspaceNameAvailableRequest,
		IsWorkspaceNameAvailableResponse,
		UpdateWorkspaceInfoRequest,
		UpdateWorkspaceInfoResponse,
		Workspace,
		WorkspaceAuditLog,
	},
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::{self, permissions},
	pin_fn,
	service,
	utils::{
		constants::request_keys,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};

mod docker_registry;
mod domain;
mod infrastructure;
#[path = "./rbac.rs"]
mod rbac_routes;

/// # Description
/// This function is used to create a sub app for every endpoint listed. It
/// creates an eve app which binds the endpoint with functions. This file
/// contains major enpoints which are meant for the workspaces, and all other
/// endpoints will come uder these
///
/// # Arguments
/// * `app` - an object of type [`App`] which contains all the configuration of
///   api including the
/// database connections.
///
/// # Returns
/// this function returns `EveApp<EveContext, EveMiddleware, App, ErrorData>`
/// containing context, middleware, object of [`App`] and Error
///
/// [`App`]: App
pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut sub_app = create_eve_app(app);

	sub_app.get(
		"/:workspaceId/info",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_workspace_info)),
		],
	);
	sub_app.post(
		"/:workspaceId/info",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT_INFO,
				api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(update_workspace_info)),
		],
	);
	sub_app.use_sub_app(
		"/:workspaceId/infrastructure",
		infrastructure::create_sub_app(app),
	);
	sub_app.use_sub_app(
		"/:workspaceId/docker-registry",
		docker_registry::create_sub_app(app),
	);
	sub_app.use_sub_app("/:workspaceId/domain", domain::create_sub_app(app));
	sub_app.use_sub_app("/:workspaceId/rbac", rbac_routes::create_sub_app(app));

	sub_app.get(
		"/is-name-available",
		[EveMiddleware::CustomFunction(pin_fn!(is_name_available))],
	);
	sub_app.post(
		"/",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(create_new_workspace)),
		],
	);
	sub_app.get(
		"/audit-log",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT_INFO,
				api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_workspace_audit_log)),
		],
	);
	sub_app.get(
		"audit-log/:resourceId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT_INFO,
				api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_resource_audit_log)),
		],
	);

	sub_app.post(
		"/:workspaceId/update-billing-info",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT_INFO,
				api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(update_billing_info)),
		],
	);

	sub_app.get(
		"/:workspaceId/promotional-credits",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT_INFO,
				api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_promotional_credits)),
		],
	);

	sub_app.get(
		"/:workspaceId/card-details",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT_INFO,
				api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_card_details)),
		],
	);

	sub_app.get(
		"/:workspaceId/subscriptions",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT_INFO,
				api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_subscriptions)),
		],
	);
	sub_app
}

/// # Description
/// This function is used to get details about an workspace
/// required inputs:
/// auth token in the authorization headers
/// workspace id in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false,
///    workspaceId: ,
///    name: ,
///    active: true or false,
///    created:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_workspace_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id_string = context
		.get_param(request_keys::WORKSPACE_ID)
		.unwrap()
		.clone();
	let workspace_id = Uuid::parse_str(&workspace_id_string)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let access_token_data = context.get_token_data().unwrap();
	let god_user_id = rbac::GOD_USER_ID.get().unwrap();

	if !access_token_data.workspaces.contains_key(&workspace_id) &&
		&access_token_data.user.id != god_user_id
	{
		Error::as_result()
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	}

	let workspace = db::get_workspace_info(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.map(|workspace| Workspace {
		id: workspace.id,
		name: workspace.name,
		active: workspace.active,
	})
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	context.success(GetWorkspaceInfoResponse { workspace });
	Ok(context)
}

/// # Description
/// This function is used to check if the workspace name is available or not
/// required inputs:
/// auth token in the authorization headers
/// ```
/// {
///     name:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false,
///    allowed: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn is_name_available(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let IsWorkspaceNameAvailableRequest { name } = context
		.get_query_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let workspace_name = name.trim().to_lowercase();

	let available = service::is_workspace_name_allowed(
		context.get_database_connection(),
		&workspace_name,
		false,
	)
	.await?;

	context.success(IsWorkspaceNameAvailableResponse { available });
	Ok(context)
}

/// # Description
/// This function is used to create new workspace
/// required inputs:
/// auth token in the authorization headers
/// ```
/// {
///     workspaceName:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false,
///    workspaceId:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn create_new_workspace(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let CreateNewWorkspaceRequest { workspace_name } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let workspace_name = workspace_name.trim().to_lowercase();

	let config = context.get_state().config.clone();

	let user_id = context.get_token_data().unwrap().user.id.clone();
	let workspace_id = service::create_workspace(
		context.get_database_connection(),
		&workspace_name,
		&user_id,
		false,
		&config,
	)
	.await?;

	context.success(CreateNewWorkspaceResponse { workspace_id });
	Ok(context)
}

/// # Description
/// This function is used to update the workspace details
/// required inputs:
/// auth token in the authorization headers
/// workspace id in the url
/// ```
/// {
///     name:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn update_workspace_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let UpdateWorkspaceInfoRequest { name, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let name = name.trim().to_lowercase();

	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	if name.is_empty() {
		// No parameters to update
		Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
	}

	let allowed = service::is_workspace_name_allowed(
		context.get_database_connection(),
		&name,
		false,
	)
	.await?;
	if !allowed {
		Error::as_result()
			.status(400)
			.body(error!(INVALID_WORKSPACE_NAME).to_string())?;
	}

	db::update_workspace_name(
		context.get_database_connection(),
		&workspace_id,
		&name,
	)
	.await?;

	context.success(UpdateWorkspaceInfoResponse {});
	Ok(context)
}

/// # Description
/// This function is used to retrieve the list of workspace audit logs
/// required inputs:
/// auth token in the authorization headers
/// workspace id in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
///    workspaceAuditLogs: []
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_workspace_audit_log(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let workspace_audit_logs = db::get_workspace_audit_log(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.map(|log| WorkspaceAuditLog {
		id: log.id,
		date: log.date,
		ip_address: log.ip_address,
		workspace_id: log.workspace_id,
		user_id: log.user_id,
		login_id: log.login_id,
		resource_id: log.resource_id,
		action: log.action,
		request_id: log.request_id,
		metadata: log.metadata,
		patr_action: log.patr_action,
		request_success: log.success,
	})
	.collect();

	context.success(GetWorkspaceAuditLogResponse {
		audit_logs: workspace_audit_logs,
	});
	Ok(context)
}

/// # Description
/// This function is used to retrieve the list resource audit logs
/// required inputs:
/// auth token in the authorization headers
/// workspace id in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
///    workspaceAuditLogs: []
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_resource_audit_log(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let resource_id = context.get_param(request_keys::RESOURCE_ID).unwrap();
	let resource_id = Uuid::parse_str(resource_id).unwrap();

	let workspace_audit_logs = db::get_resource_audit_log(
		context.get_database_connection(),
		&resource_id,
	)
	.await?
	.into_iter()
	.map(|log| WorkspaceAuditLog {
		id: log.id,
		date: log.date,
		ip_address: log.ip_address,
		workspace_id: log.workspace_id,
		user_id: log.user_id,
		login_id: log.login_id,
		resource_id: log.resource_id,
		action: log.action,
		request_id: log.request_id,
		metadata: log.metadata,
		patr_action: log.patr_action,
		request_success: log.success,
	})
	.collect();

	context.success(GetWorkspaceAuditLogResponse {
		audit_logs: workspace_audit_logs,
	});
	Ok(context)
}

/// # Description
/// This function is used to update the workspace details
/// required inputs:
/// auth token in the authorization headers
/// workspace id in the url
/// ```
/// {
///   firstName: "",
///   lastName: "",
///   email: "",
///   phone: "",
///   addressDetails: {
///                         addressLine1: "",
///                         addressLine2: "",
///                         city: "",
///                         state: "",
///                         country: "",
///                         zip: ""
///                     },
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn update_billing_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let UpdateBillingInfoRequest {
		first_name,
		last_name,
		email,
		phone,
		address_details,
		..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let config = context.get_state().config.clone();

	service::update_billing_info(
		&workspace_id,
		first_name,
		last_name,
		email,
		phone,
		address_details,
		&config,
	)
	.await?;

	context.success(UpdateBillingInfoResponse {});
	Ok(context)
}

/// # Description
/// This function is used to fetch the promotional credits of the workspace
/// required inputs:
/// auth token in the authorization headers
/// workspace id in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///
///    success: true or false
///    list: [
///              {
///                  id:
///                  customerId:
///                  type:
///                  amount:
///                  description:
///                  creditType:
///                  closingBalance:
///                  createdAt:
///                  closingBalance:
///                  createdAt:
///                  object:
///                  currencyCode:
///              }
///          ]
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_promotional_credits(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let config = context.get_state().config.clone();

	let credit_balance = service::get_credit_balance(&workspace_id, &config)
		.await?
		.list
		.into_iter()
		.map(|credit| PromotionalCredits {
			id: credit.promotional_credit.id,
			customer_id: credit.promotional_credit.customer_id,
			r#type: credit.promotional_credit.r#type,
			amount: credit.promotional_credit.amount,
			description: credit.promotional_credit.description,
			credit_type: credit.promotional_credit.credit_type,
			closing_balance: credit.promotional_credit.closing_balance,
			created_at: credit.promotional_credit.created_at,
			object: credit.promotional_credit.object,
			currency_code: credit.promotional_credit.currency_code,
		})
		.collect();

	context.success(GetCreditBalanceResponse {
		list: credit_balance,
	});
	Ok(context)
}

/// # Description
/// This function is used to fetch the promotional credits of the workspace
/// required inputs:
/// auth token in the authorization headers
/// workspace id in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///
///    success: true or false
///    list: [
///            {
///                id:,
///                updatedAt:,
///                deleted:,
///                object:,
///                customerId:,
///                type:,
///                referenceId:,
///                status:,
///                gateway:,
///                gatewayAccountId:,
///                createdAt:,
///                card:
///                {
///                    firstName:,
///                    lastName:,
///                    iin:,
///                    last4:,
///                    fundingType:,
///                    expiryMonth:,
///                    expiryYear:,
///                    billingAddr1:,
///                    billingAddre2:,
///                    billingCity:,
///                    billingState:,
///                    billingZip:,
///                    maskedNumber:,
///                    object:,
///                    brand:
///                }
///             }
///           ]
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_card_details(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let config = context.get_state().config.clone();

	let card_details = service::get_card_details(&config, &workspace_id)
		.await?
		.list
		.into_iter()
		.map(|payment_source| PaymentSource {
			id: payment_source.payment_source.id,
			updated_at: payment_source.payment_source.updated_at,
			deleted: payment_source.payment_source.deleted,
			object: payment_source.payment_source.object,
			customer_id: payment_source.payment_source.customer_id,
			r#type: payment_source.payment_source.r#type,
			reference_id: payment_source.payment_source.reference_id,
			status: payment_source.payment_source.status,
			gateway: payment_source.payment_source.gateway,
			gateway_account_id: payment_source
				.payment_source
				.gateway_account_id,
			created_at: payment_source.payment_source.created_at,
			card: payment_source.payment_source.card.map(|card| Card {
				first_name: card.first_name,
				last_name: card.last_name,
				iin: card.iin,
				last4: card.last4,
				funding_type: card.funding_type,
				expiry_month: card.expiry_month,
				expiry_year: card.expiry_year,
				billing_addr1: card.billing_addr1,
				billing_addre2: card.billing_addre2,
				billing_city: card.billing_city,
				billing_state: card.billing_state,
				billing_zip: card.billing_zip,
				masked_number: card.masked_number,
				object: card.object,
				brand: card.brand,
			}),
		})
		.collect();

	context.success(GetCardDetailsResponse { list: card_details });
	Ok(context)
}

/// # Description
/// This function is used to fetch the promotional credits of the workspace
/// required inputs:
/// auth token in the authorization headers
/// workspace id in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///
///    success: true or false
///    list: [
///              {
///                  id:
///                  billingPeriod:
///                  billing_period_unit:
///                  customerId:
///                  status:
///                  current_term_start:
///                  current_term_end:
///                  next_billing_at:
///                  type:
///                  amount:
///                  description:
///                  creditType:
///                  closingBalance:
///                  createdAt:
///                  closingBalance:
///                  createdAt:
///                  object:
///                  currencyCode:
///              }
///          ]
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_subscriptions(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let config = context.get_state().config.clone();
	let subscriptions = service::get_subscriptions(&config, &workspace_id)
		.await?
		.list
		.into_iter()
		.map(|subscription| Subscription {
			id: subscription.subscription.id,
			billing_period: subscription.subscription.billing_period,
			billing_period_unit: subscription.subscription.billing_period_unit,
			customer_id: subscription.subscription.customer_id,
			status: subscription.subscription.status,
			current_term_start: subscription.subscription.current_term_start,
			current_term_end: subscription.subscription.current_term_end,
			next_billing_at: subscription.subscription.next_billing_at,
			created_at: subscription.subscription.created_at,
			started_at: subscription.subscription.started_at,
			activated_at: subscription.subscription.activated_at,
			cancelled_at: subscription.subscription.cancelled_at,
			updated_at: subscription.subscription.updated_at,
			has_scheduled_changes: subscription
				.subscription
				.has_scheduled_changes,
			channel: subscription.subscription.channel,
			object: subscription.subscription.object,
			currency_code: subscription.subscription.currency_code,
			subscription_items: subscription
				.subscription
				.subscription_items
				.into_iter()
				.map(|subscription_item| SubscriptionItem {
					item_price_id: subscription_item.item_price_id,
					item_type: subscription_item.item_type,
					quantity: subscription_item.quantity,
					unit_price: subscription_item.unit_price,
					amount: subscription_item.amount,
					object: subscription_item.object,
				})
				.collect(),
			due_invoices_count: subscription.subscription.due_invoices_count,
		})
		.collect();

	context.success(GetSubscriptionsResponse {
		list: subscriptions,
	});
	Ok(context)
}
