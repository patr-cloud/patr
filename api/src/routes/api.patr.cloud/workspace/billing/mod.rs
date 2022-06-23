use api_models::{
	models::workspace::billing::{
		AddBillingAddressRequest,
		AddBillingAddressResponse,
		AddCreditsRequest,
		AddCreditsResponse,
		AddPaymentMethodResponse,
		Address,
		ConfirmPaymentMethodRequest,
		ConfirmPaymentMethodResponse,
		ConfirmPaymentRequest,
		ConfirmPaymentResponse,
		DeleteBillingAddressResponse,
		DeletePaymentMethodResponse,
		GetBillingAddressResponse,
		GetCardDetailsResponse,
		GetCreditBalanceResponse,
		GetCreditsResponse,
		GetCurrentBillResponse,
		GetSubscriptionsResponse,
		PaymentMethod,
		PromotionalCredits,
		Subscription,
		SubscriptionItem,
		UpdateBillingAddressRequest,
		UpdateBillingAddressResponse,
	},
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::permissions,
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

	sub_app.post(
		"/billing-address",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT,
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
			EveMiddleware::CustomFunction(pin_fn!(add_billing_address)),
		],
	);

	sub_app.patch(
		"/billing-address",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT,
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
			EveMiddleware::CustomFunction(pin_fn!(update_billing_address)),
		],
	);

	sub_app.delete(
		"/billing-address",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT,
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
			EveMiddleware::CustomFunction(pin_fn!(delete_billing_address)),
		],
	);

	sub_app.get(
		"/promotional-credits",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT,
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

	sub_app.post(
		"/payment-method",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT,
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
			EveMiddleware::CustomFunction(pin_fn!(add_payment_method)),
		],
	);

	sub_app.get(
		"/payment-method",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT,
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
			EveMiddleware::CustomFunction(pin_fn!(get_payment_method)),
		],
	);

	sub_app.delete(
		"/payment-method/:paymentMethodId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT,
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
			EveMiddleware::CustomFunction(pin_fn!(delete_payment_method)),
		],
	);

	sub_app.post(
		"/confirm-payment-method",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT,
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
			EveMiddleware::CustomFunction(pin_fn!(confirm_payment_method)),
		],
	);

	sub_app.get(
		"/subscriptions",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT,
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

	sub_app.get(
		"/billing-address",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT,
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
			EveMiddleware::CustomFunction(pin_fn!(get_billing_address)),
		],
	);

	sub_app.post(
		"/add-credits",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT,
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
			EveMiddleware::CustomFunction(pin_fn!(add_credits)),
		],
	);

	sub_app.get(
		"/get-credits",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT,
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
			EveMiddleware::CustomFunction(pin_fn!(get_credits)),
		],
	);

	sub_app.post(
		"/confirm-payment",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT,
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
			EveMiddleware::CustomFunction(pin_fn!(confirm_payment)),
		],
	);

	sub_app.get(
		"/get-current-bill",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT,
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
			EveMiddleware::CustomFunction(pin_fn!(get_current_bill)),
		],
	);

	sub_app
}

async fn add_billing_address(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let AddBillingAddressRequest {
		address_details, ..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let config = context.get_state().config.clone();

	service::add_billing_address(
		context.get_database_connection(),
		&workspace_id,
		address_details,
		&config,
	)
	.await?;

	context.success(AddBillingAddressResponse {});
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
async fn update_billing_address(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let UpdateBillingAddressRequest {
		address_details, ..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	if let Some(address_info) = address_details {
		service::update_billing_address(
			context.get_database_connection(),
			&workspace_id,
			address_info,
		)
		.await?;
	}

	context.success(UpdateBillingAddressResponse {});
	Ok(context)
}

async fn delete_billing_address(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();
	let billing_address_id = db::get_workspace_info(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?
	.address_id
	.status(404)
	.body(error!(ADDRESS_NOT_FOUND).to_string())?;

	db::delete_billing_address_from_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?;
	db::delete_billing_address(
		context.get_database_connection(),
		&billing_address_id,
	)
	.await?;
	context.success(DeleteBillingAddressResponse {});
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
/// This function is used to fetch the card details of the workspace
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
async fn get_payment_method(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let config = context.get_state().config.clone();

	let card_details = service::get_card_details(
		context.get_database_connection(),
		&workspace_id,
		&config,
	)
	.await?
	.into_iter()
	.map(|payment_source| PaymentMethod {
		id: payment_source.id,
		customer: payment_source.customer,
		r#type: payment_source.r#type,
		card: payment_source.card,
		created: payment_source.created,
	})
	.collect();

	context.success(GetCardDetailsResponse { list: card_details });
	Ok(context)
}

/// # Description
/// This function is used to add the card details to the workspace
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
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn add_payment_method(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();
	let config = context.get_state().config.clone();
	let client_secret = service::add_card_details(
		context.get_database_connection(),
		&workspace_id,
		&config,
	)
	.await?
	.client_secret;
	context.success(AddPaymentMethodResponse { client_secret });
	Ok(context)
}

async fn delete_payment_method(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();
	let payment_method_id = context
		.get_param(request_keys::PAYMENT_METHOD_ID)
		.unwrap()
		.parse::<String>()?;

	let _ = db::get_payment_method_info(
		context.get_database_connection(),
		&payment_method_id,
	)
	.await?
	.status(400)
	.body(error!(WRONG_PARAMETERS).to_string())?;
	let config = context.get_state().config.clone();
	service::delete_payment_method(
		context.get_database_connection(),
		&workspace_id,
		&payment_method_id,
		&config,
	)
	.await?;
	context.success(DeletePaymentMethodResponse {});
	Ok(context)
}

async fn confirm_payment_method(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();
	let ConfirmPaymentMethodRequest {
		payment_method_id, ..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	db::add_payment_method_info(
		context.get_database_connection(),
		&workspace_id,
		&payment_method_id,
	)
	.await?;
	if db::get_workspace_info(context.get_database_connection(), &workspace_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?
		.default_payment_method_id
		.is_none()
	{
		db::set_default_payment_method_for_workspace(
			context.get_database_connection(),
			&workspace_id,
			&payment_method_id,
		)
		.await?;
	}
	context.success(ConfirmPaymentMethodResponse {});
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

async fn get_billing_address(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let user_address = db::get_workspace_info(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(500)?
	.address_id;
	let address = if let Some(addr_id) = user_address {
		let billing_address = db::get_billing_address(
			context.get_database_connection(),
			&addr_id,
		)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

		Some(Address {
			first_name: billing_address.first_name,
			last_name: billing_address.last_name,
			address_line_1: billing_address.address_line_1,
			address_line_2: billing_address.address_line_2,
			address_line_3: billing_address.address_line_3,
			city: billing_address.city,
			state: billing_address.state,
			zip: billing_address.zip,
			country: billing_address.country,
		})
	} else {
		None
	};
	context.success(GetBillingAddressResponse { address });
	Ok(context)
}

async fn add_credits(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let AddCreditsRequest { credits, .. } =
		context
			.get_body_as()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();
	let secret_id = service::add_credits_to_workspace(
		context.get_database_connection(),
		&workspace_id,
		credits,
		&config,
	)
	.await?;

	context.success(AddCreditsResponse {
		client_secret: secret_id,
	});
	Ok(context)
}

async fn get_credits(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let credits = db::get_credits_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.map(|transaction| transaction.amount.abs())
	.sum::<f64>()
	.max(0f64);

	context.success(GetCreditsResponse { credits });
	Ok(context)
}

async fn confirm_payment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let ConfirmPaymentRequest {
		payment_intent_id, ..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();

	let success = service::confirm_payment_method(
		context.get_database_connection(),
		&workspace_id,
		&payment_intent_id,
		&config,
	)
	.await?;

	if success {
		context.success(ConfirmPaymentResponse {});
	} else {
		context.json(error!(PAYMENT_FAILED));
	}
	Ok(context)
}

async fn get_current_bill(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let workspace = db::get_workspace_info(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	context.success(GetCurrentBillResponse {
		bill: workspace.amount_due as u64,
	});
	Ok(context)
}
