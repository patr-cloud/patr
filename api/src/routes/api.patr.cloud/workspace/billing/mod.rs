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
	utils::Error,
};

pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::billing::billing_address::ADD,
				|AddBillingAddressPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			)
			.disallow_api_token(),
			app.clone(),
			add_billing_address,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::billing::billing_address::EDIT,
				|UpdateBillingAddressPath { workspace_id },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			update_billing_address,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::billing::billing_address::DELETE,
				|DeleteBillingAddressPath { workspace_id },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			)
			.disallow_api_token(),
			app.clone(),
			delete_billing_address,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::billing::payment_method::ADD,
				|AddPaymentMethodPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			add_payment_method,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::billing::payment_method::LIST,
				|GetPaymentMethodPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			)
			.disallow_api_token(),
			app.clone(),
			get_payment_method,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::billing::payment_method::DELETE,
				|DeletePaymentMethodPath { workspace_id, .. },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			delete_payment_method,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::billing::payment_method::ADD,
				|ConfirmPaymentMethodPath { workspace_id, .. },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			)
			.disallow_api_token(),
			app.clone(),
			confirm_payment_method,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::billing::payment_method::ADD,
				|ConfirmPaymentMethodPath { workspace_id, .. },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			confirm_payment_method,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::billing::payment_method::EDIT,
				|SetPrimaryCardPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			)
			.disallow_api_token(),
			app.clone(),
			set_primary_card,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::billing::billing_address::INFO,
				|GetBillingAddressPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			get_billing_address,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::billing::MAKE_PAYMENT,
				|ConfirmPaymentMethodPath { workspace_id, .. },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			)
			.disallow_api_token(),
			app.clone(),
			add_credits,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::billing::MAKE_PAYMENT,
				|ConfirmCreditsPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			confirm_credits,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::billing::MAKE_PAYMENT,
				|MakePaymentPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			)
			.disallow_api_token(),
			app.clone(),
			make_payment,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::billing::INFO,
				|GetCurrentUsagePath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			)
			.disallow_api_token(),
			app.clone(),
			get_current_bill,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::billing::INFO,
				|GetBillBreakdownPath { workspace_id }, (), app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			get_bill_breakdown,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::billing::INFO,
				|GetTransactionHistoryPath { workspace_id },
				 (),
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id).await
				},
			),
			app.clone(),
			get_transaction_history,
		)
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
		path: GetCurrentUsagePath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<GetCurrentUsageRequest>,
) -> Result<GetCurrentUsageResponse, Error> {
	let current_month_bill_so_far =
		db::get_workspace_info(&mut connection, &workspace_id)
			.await?
			.ok_or_else(|| ErrorType::WrongParameters)?
			.amount_due_in_cents;

	let current_month_bill_so_far =
		TotalAmount::NeedToPay(current_month_bill_so_far);

	let leftover_credits_or_due =
		db::get_total_amount_in_cents_to_pay_for_workspace(
			&mut connection,
			&workspace_id,
		)
		.await?;

	Ok(GetCurrentUsageResponse {
		current_usage: current_month_bill_so_far + leftover_credits_or_due,
	});
}

async fn get_bill_breakdown(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GetBillBreakdownPath { workspace_id },
		query: (),
		body: GetBillBreakdownRequest { month, year },
	}: DecodedRequest<GetBillBreakdownRequest>,
) -> Result<GetBillBreakdownResponse, Error> {
	let request_id = Uuid::new_v4();

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

	Ok(GetBillBreakdownResponse { bill });
}

async fn get_transaction_history(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GetTransactionHistoryPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<GetTransactionHistoryRequest>,
) -> Result<GetTransactionHistoryResponse, Error> {
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

	Ok(GetTransactionHistoryResponse { transactions });
}
