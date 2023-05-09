use std::{cmp::min, ops::Sub};

use api_models::{
	models::prelude::*,
	utils::{DateTime, Uuid},
};
use axum::{extract::State, Router};
use chrono::{Duration, TimeZone, Utc};

use crate::{
	app::App,
	db,
	error,
	models::rbac::permissions,
	prelude::*,
	service,
	utils::{constants::request_keys, Error},
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
pub fn create_sub_app(app: &App) -> Router<App> {
	let mut sub_app = create_axum_router(app);

	sub_app.post(
		"/billing-address",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission:
					permissions::workspace::billing::billing_address::ADD,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;
					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;
					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}
					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(add_billing_address)),
		],
	);

	sub_app.patch(
		"/billing-address",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission:
					permissions::workspace::billing::billing_address::EDIT,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(update_billing_address)),
		],
	);

	sub_app.delete(
		"/billing-address",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission:
					permissions::workspace::billing::billing_address::DELETE,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(delete_billing_address)),
		],
	);

	sub_app.post(
		"/payment-method",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission:
					permissions::workspace::billing::payment_method::ADD,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(add_payment_method)),
		],
	);

	sub_app.get(
		"/payment-method",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission:
					permissions::workspace::billing::payment_method::LIST,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(get_payment_method)),
		],
	);

	sub_app.delete(
		"/payment-method/:paymentMethodId",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission:
					permissions::workspace::billing::payment_method::DELETE,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(delete_payment_method)),
		],
	);

	sub_app.post(
		"/confirm-payment-method",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission:
					permissions::workspace::billing::payment_method::ADD,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(confirm_payment_method)),
		],
	);

	sub_app.post(
		"/set-primary-card",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission:
					permissions::workspace::billing::payment_method::EDIT,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(set_primary_card)),
		],
	);

	sub_app.get(
		"/billing-address",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::billing::billing_address::INFO,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(get_billing_address)),
		],
	);

	sub_app.post(
		"/add-credits",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission: permissions::workspace::billing::MAKE_PAYMENT,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(add_credits)),
		],
	);

	sub_app.post(
		"/confirm-credits",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission: permissions::workspace::billing::MAKE_PAYMENT,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(confirm_credits)),
		],
	);

	sub_app.post(
		"/make-payment",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission: permissions::workspace::billing::MAKE_PAYMENT,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(make_payment)),
		],
	);

	sub_app.post(
		"/confirm-payment",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission: permissions::workspace::billing::MAKE_PAYMENT,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(confirm_payment)),
		],
	);

	sub_app.get(
		"/get-current-usage",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::billing::INFO,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(get_current_bill)),
		],
	);

	sub_app.get(
		"/bill-breakdown",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::billing::INFO,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(get_bill_breakdown)),
		],
	);

	sub_app.get(
		"/transaction-history",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::billing::INFO,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource =
						db::get_resource_by_id(&mut connection, &workspace_id)
							.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(get_transaction_history)),
		],
	);

	sub_app
}

async fn add_billing_address(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: AddBillingAddressPath { workspace_id },
		query: (),
		body: AddBillingAddressRequest { address_details },
	}: DecodedRequest<AddBillingAddressRequest>,
) -> Result<(), Error> {
	service::add_billing_address(
		&mut connection,
		&workspace_id,
		address_details,
		&config,
	)
	.await?;

	Ok(())
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
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: UpdateBillingAddressPath { workspace_id },
		query: (),
		body: UpdateBillingAddressRequest { address_details },
	}: DecodedRequest<UpdateBillingAddressRequest>,
) -> Result<(), Error> {
	if let Some(address_info) = address_details {
		service::update_billing_address(
			&mut connection,
			&workspace_id,
			address_info,
		)
		.await?;
	}

	Ok(())
}

async fn delete_billing_address(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: DeleteBillingAddressPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<DeleteBillingAddressRequest>,
) -> Result<(), Error> {
	let billing_address_id =
		db::get_workspace_info(&mut connection, &workspace_id)
			.await?
			.ok_or_else(|| ErrorType::internal_error())?
			.address_id
			.ok_or_else(|| ErrorType::NotFound)?;

	db::delete_billing_address_from_workspace(&mut connection, &workspace_id)
		.await?;
	db::delete_billing_address(&mut connection, &billing_address_id).await?;
	Ok(())
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
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GetPaymentMethodPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<GetPaymentMethodRequest>,
) -> Result<GetPaymentMethodResponse, Error> {
	let card_details =
		service::get_card_details(&mut connection, &workspace_id, &config)
			.await?
			.into_iter()
			.map(|payment_source| PaymentMethod {
				id: payment_source.id,
				customer: payment_source.customer,
				card: payment_source.card,
				created: payment_source.created,
			})
			.collect();

	Ok(GetPaymentMethodResponse { list: card_details })
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
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: AddPaymentMethodPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<AddPaymentMethodRequest>,
) -> Result<AddPaymentMethodResponse, Error> {
	let client_secret =
		service::add_card_details(&mut connection, &workspace_id, &config)
			.await?
			.client_secret
			.ok_or_else(|| ErrorType::internal_error())?;
	Ok(AddPaymentMethodResponse { client_secret })
}

async fn delete_payment_method(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path:
			DeletePaymentMethodPath {
				workspace_id,
				payment_method_id,
			},
		query: (),
		body: (),
	}: DecodedRequest<DeletePaymentMethodRequest>,
) -> Result<(), Error> {
	let _ = db::get_payment_method_info(&mut connection, &payment_method_id)
		.await?
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	service::delete_payment_method(
		&mut connection,
		&workspace_id,
		&payment_method_id,
		&config,
	)
	.await?;

	Ok(())
}

async fn confirm_payment_method(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: ConfirmPaymentMethodPath { workspace_id },
		query: (),
		body: ConfirmPaymentMethodRequest { payment_method_id },
	}: DecodedRequest<ConfirmPaymentMethodRequest>,
) -> Result<(), Error> {
	db::add_payment_method_info(
		&mut connection,
		&workspace_id,
		&payment_method_id,
	)
	.await?;
	if db::get_workspace_info(&mut connection, &workspace_id)
		.await?
		.ok_or_else(|| ErrorType::internal_error())?
		.default_payment_method_id
		.is_none()
	{
		db::set_default_payment_method_for_workspace(
			&mut connection,
			&workspace_id,
			&payment_method_id,
		)
		.await?;
	}
	Ok(())
}

async fn set_primary_card(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: SetPrimaryCardPath { workspace_id },
		query: (),
		body: SetPrimaryCardRequest { payment_method_id },
	}: DecodedRequest<SetPrimaryCardRequest>,
) -> Result<(), Error> {
	// Check if payment method that is requested to be default, exists or not
	db::get_payment_method_info(&mut connection, &payment_method_id)
		.await?
		.ok_or_else(|| ErrorType::internal_error())?;

	// Set payment method to default in workspace
	db::set_default_payment_method_for_workspace(
		&mut connection,
		&workspace_id,
		&payment_method_id,
	)
	.await?;
	Ok(())
}

async fn get_billing_address(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GetBillingAddressPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<GetBillingAddressRequest>,
) -> Result<GetBillingAddressResponse, Error> {
	let user_address = db::get_workspace_info(&mut connection, &workspace_id)
		.await?
		.ok_or_else(|| ErrorType::internal_error())?
		.address_id;
	let address = if let Some(addr_id) = user_address {
		let billing_address =
			db::get_billing_address(&mut connection, &addr_id)
				.await?
				.ok_or_else(|| ErrorType::internal_error())?;

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
	Ok(GetBillingAddressResponse { address })
}

async fn add_credits(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: AddCreditsPath { workspace_id },
		query: (),
		body: AddCreditsRequest {
			credits,
			payment_method_id,
		},
	}: DecodedRequest<AddCreditsRequest>,
) -> Result<AddCreditsResponse, Error> {
	let (transaction_id, client_secret) = service::add_credits_to_workspace(
		&mut connection,
		&workspace_id,
		credits,
		&payment_method_id,
		&config,
	)
	.await?;

	Ok(AddCreditsResponse {
		transaction_id,
		client_secret,
	})
}

async fn confirm_credits(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: ConfirmCreditsPath { workspace_id },
		query: (),
		body: ConfirmCreditsRequest { transaction_id },
	}: DecodedRequest<ConfirmCreditsRequest>,
) -> Result<(), Error> {
	let transaction = db::get_transaction_by_transaction_id(
		&mut connection,
		&workspace_id,
		&transaction_id,
	)
	.await?
	.ok_or_else(|| ErrorType::WrongParameters)?;

	match transaction.payment_status {
		PaymentStatus::Success => {
			return Ok(());
		}
		PaymentStatus::Failed => {
			return Err(ErrorType::PaymentFailed.into());
		}
		PaymentStatus::Pending => {
			// in pending state so need to confirm it
		}
	}

	let success = service::confirm_payment(
		&mut connection,
		&workspace_id,
		&transaction_id,
		&config,
	)
	.await?;

	if success {
		service::send_purchase_credits_success_email(
			&mut connection,
			&workspace_id,
			&transaction_id,
		)
		.await?;

		let current_amount_due =
			db::get_workspace_info(&mut connection, &workspace_id)
				.await?
				.ok_or_else(|| ErrorType::internal_error())?
				.amount_due_in_cents;
		let bill_after_payment = TotalAmount::NeedToPay(current_amount_due) +
			db::get_total_amount_in_cents_to_pay_for_workspace(
				&mut connection,
				&workspace_id,
			)
			.await?;

		if let TotalAmount::NeedToPay(_bill) = bill_after_payment {
			service::send_bill_paid_using_credits_email(
				&mut connection,
				&workspace_id,
				&transaction_id,
			)
			.await?;
		}

		Ok(())
	} else {
		Err(ErrorType::PaymentFailed.into())
	}
}

async fn make_payment(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: MakePaymentPath { workspace_id },
		query: (),
		body: MakePaymentRequest {
			amount,
			payment_method_id,
		},
	}: DecodedRequest<MakePaymentRequest>,
) -> Result<MakePaymentResponse, Error> {
	let (transaction_id, client_secret) = service::make_payment(
		&mut connection,
		&workspace_id,
		amount,
		&payment_method_id,
		&config,
	)
	.await?;

	Ok(MakePaymentResponse {
		transaction_id,
		client_secret,
	})
}

async fn confirm_payment(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: ConfirmPaymentPath { workspace_id },
		query: (),
		body: ConfirmPaymentRequest { transaction_id },
	}: DecodedRequest<ConfirmPaymentRequest>,
) -> Result<(), Error> {
	let transaction = db::get_transaction_by_transaction_id(
		&mut connection,
		&workspace_id,
		&transaction_id,
	)
	.await?
	.ok_or_else(|| ErrorType::WrongParameters)?;

	match transaction.payment_status {
		PaymentStatus::Success => return Ok(()),
		PaymentStatus::Failed => {
			return Err(ErrorType::PaymentFailed.into());
		}
		PaymentStatus::Pending => {
			// in pending state so need to confirm it
		}
	}

	let success = service::confirm_payment(
		&mut connection,
		&workspace_id,
		&transaction_id,
		&config,
	)
	.await?;

	if success {
		service::send_partial_payment_success_email(
			&mut connection,
			&workspace_id,
			&transaction_id,
		)
		.await?;
		Ok(())
	} else {
		Err(ErrorType::PaymentFailed.into())
	}
}

async fn get_current_bill(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GetDockerRepositoryImageDetailsPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<GetDockerRepositoryImageDetailsRequest>,
) -> Result<GetDockerRepositoryImageDetailsResponse, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let current_month_bill_so_far =
		db::get_workspace_info(&mut connection, &workspace_id)
			.await?
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?
			.amount_due_in_cents;
	let current_month_bill_so_far =
		TotalAmount::NeedToPay(current_month_bill_so_far);

	let leftover_credits_or_due =
		db::get_total_amount_in_cents_to_pay_for_workspace(
			&mut connection,
			&workspace_id,
		)
		.await?;

	context.success(GetCurrentUsageResponse {
		current_usage: current_month_bill_so_far + leftover_credits_or_due,
	});
	Ok(context)
}

async fn get_bill_breakdown(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GetDockerRepositoryImageDetailsPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<GetDockerRepositoryImageDetailsRequest>,
) -> Result<GetDockerRepositoryImageDetailsResponse, Error> {
	let request_id = Uuid::new_v4();

	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let GetBillBreakdownRequest { month, year, .. } = context
		.get_query_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	log::trace!(
		"request_id: {} getting bill breakdown for month: {} and year: {}",
		request_id,
		month,
		year
	);

	let month_start_date = Utc
		.with_ymd_and_hms(year as i32, month, 1, 0, 0, 0)
		.unwrap();
	let month_end_date = Utc
		.with_ymd_and_hms(
			(if month == 12 { year + 1 } else { year }) as i32,
			if month == 12 { 1 } else { month + 1 },
			1,
			0,
			0,
			0,
		)
		.unwrap()
		.sub(Duration::nanoseconds(1));
	let month_end_date = min(month_end_date, Utc::now());

	let bill = service::get_total_resource_usage(
		&mut connection,
		&workspace_id,
		&month_start_date,
		&month_end_date,
		year,
		month,
		&request_id,
	)
	.await?;

	context.success(GetBillBreakdownResponse { bill });
	Ok(context)
}

async fn get_transaction_history(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GetDockerRepositoryImageDetailsPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<GetDockerRepositoryImageDetailsRequest>,
) -> Result<GetDockerRepositoryImageDetailsResponse, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let transactions =
		db::get_transactions_in_workspace(&mut connection, &workspace_id)
			.await?
			.into_iter()
			.map(|transaction| Transaction {
				id: transaction.id,
				month: transaction.month,
				amount: transaction.amount_in_cents as u64,
				payment_intent_id: transaction.payment_intent_id,
				date: DateTime(transaction.date),
				workspace_id: transaction.workspace_id,
				payment_status: transaction.payment_status,
				description: transaction.description,
				transaction_type: transaction.transaction_type,
			})
			.collect();

	context.success(GetTransactionHistoryResponse { transactions });

	Ok(context)
}
