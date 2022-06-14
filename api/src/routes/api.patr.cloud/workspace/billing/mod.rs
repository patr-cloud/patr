use api_models::{
	models::workspace::billing::{
		AddCouponRequest,
		AddCouponResponse,
		AddPaymentSourceResponse,
		Address,
		ConfirmCardDetailsRequest,
		ConfirmCardDetailsResponse,
		DeleteBillingAddressResponse,
		DeletePaymentMethodResponse,
		GetBillingAddressResponse,
		GetCardDetailsResponse,
		GetCreditBalanceResponse,
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
	models::{rbac::permissions, ProductLimits},
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
		"/update-billing-address",
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
		"/confirm-card-details",
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
			EveMiddleware::CustomFunction(pin_fn!(confirm_card_details)),
		],
	);

	sub_app.delete(
		"/:paymentMethodId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::DELETE,
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
		"/coupon",
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
			EveMiddleware::CustomFunction(pin_fn!(add_coupon_to_workspace)),
		],
	);

	sub_app.delete(
		"/billing-address/:billingAddressId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::DELETE,
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

	service::update_billing_address(
		context.get_database_connection(),
		&workspace_id,
		address_details,
	)
	.await?;

	context.success(UpdateBillingAddressResponse {});
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

	let credit_balance = db::get_credit_balance(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.credits;

	context.success(GetCreditBalanceResponse {
		credits: credit_balance,
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

	let card_details = service::get_card_details(
		context.get_database_connection(),
		&workspace_id,
		&config,
	)
	.await?;

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

	let client_secret = service::add_card_details(&workspace_id, &config)
		.await?
		.client_secret;

	context.success(AddPaymentSourceResponse { client_secret });

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
		let billing_address =
			db::get_address_by_id(context.get_database_connection(), &addr_id)
				.await?
				.status(500)
				.body(error!(SERVER_ERROR).to_string())?;
		Some(Address {
			first_name: billing_address.first_name,
			last_name: billing_address.last_name,
			address_line1: billing_address.address_line_1,
			address_line2: billing_address.address_line_2,
			address_line3: billing_address.address_line_3,
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

async fn confirm_card_details(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let ConfirmCardDetailsRequest {
		payment_method_id,
		status,
		..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	db::add_payment_method_info(
		context.get_database_connection(),
		&workspace_id,
		&payment_method_id,
		status,
	)
	.await?;

	if db::get_workspace_info(context.get_database_connection(), &workspace_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?
		.primary_payment_method
		.is_none()
	{
		db::set_primary_payment_method_for_workspace(
			context.get_database_connection(),
			&workspace_id,
			&payment_method_id,
		)
		.await?;
	}

	service::upgrade_limits_for_workspace(
		context.get_database_connection(),
		&workspace_id,
		&ProductLimits {
			deployment: 10,
			static_site: 100,
			managed_database: 5,
			managed_url: 100,
			domain: 5,
			secret: 100,
		},
	)
	.await?;

	context.success(ConfirmCardDetailsResponse {});
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

async fn add_coupon_to_workspace(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let AddCouponRequest { coupon_name, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let coupon =
		db::get_coupon_by_name(context.get_database_connection(), &coupon_name)
			.await?
			.status(400)
			.body(error!(INVALID_COUPON).to_string())?;

	let current_coupon = db::get_active_coupon_by_id(
		context.get_database_connection(),
		&workspace_id,
		&coupon.id,
	)
	.await?;

	if let Some(current_coupon) = current_coupon {
		if current_coupon.remaining_usage == 0 {
			return Error::as_result()
				.status(400)
				.body(error!(COUPON_USED).to_string())?;
		}

		db::update_active_coupon_usage(
			context.get_database_connection(),
			&coupon.id,
			&workspace_id,
			&current_coupon.remaining_usage - 1,
		)
		.await?;
	}

	db::add_coupon_to_workspace(
		context.get_database_connection(),
		&workspace_id,
		&coupon.id,
		&coupon.num_usage,
	)
	.await?;

	context.success(AddCouponResponse {});
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
