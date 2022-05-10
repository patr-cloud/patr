use api_models::{
	models::workspace::billing::{
		AddPaymentSourceResponse,
		Address,
		Card,
		GetBillingAddressResponse,
		GetCardDetailsResponse,
		GetCreditBalanceResponse,
		GetSubscriptionsResponse,
		HostedPage,
		PaymentSource,
		PromotionalCredits,
		Subscription,
		SubscriptionItem,
		UpdateBillingInfoRequest,
		UpdateBillingInfoResponse,
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
		"/update-billing-info",
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
			EveMiddleware::CustomFunction(pin_fn!(update_billing_info)),
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

	sub_app.get(
		"/card-details",
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
			EveMiddleware::CustomFunction(pin_fn!(get_card_details)),
		],
	);

	sub_app.post(
		"/card-details",
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
			EveMiddleware::CustomFunction(pin_fn!(add_card_details)),
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

	sub_app
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
async fn get_card_details(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let config = context.get_state().config.clone();

	let card_details = service::get_card_details(&workspace_id, &config)
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
async fn add_card_details(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let config = context.get_state().config.clone();

	let hosted_page = service::add_card_details(&workspace_id, &config)
		.await?
		.hosted_page;

	context.success(AddPaymentSourceResponse {
		hosted_page: HostedPage {
			id: hosted_page.id,
			r#type: hosted_page.r#type,
			url: hosted_page.url,
			state: hosted_page.state,
			embed: hosted_page.embed,
			created_at: hosted_page.created_at,
			expires_at: hosted_page.expires_at,
			object: hosted_page.object,
			updated_at: hosted_page.updated_at,
			resource_version: hosted_page.resource_version,
		},
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

	let config = context.get_state().config.clone();
	let billing_address = service::get_billing_address(&config, &workspace_id)
		.await?
		.map(|address| Address {
			address_line1: address.address_line1,
			address_line2: address.address_line2,
			address_line3: address.address_line3,
			city: address.city,
			state: address.state,
			zip: address.zip,
			country: address.country,
		});

	context.success(GetBillingAddressResponse {
		address: billing_address,
	});
	Ok(context)
}
