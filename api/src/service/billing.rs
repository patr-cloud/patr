use api_models::utils::Uuid;
use chrono::Utc;
use eve_rs::AsError;
use reqwest::Client;
use serde_json::json;

use crate::{
	db,
	error,
	models::{PaymentIntent, PaymentIntentObject, PaymentMethodStatus},
	utils::{settings::Settings, Error},
	Database,
};

pub async fn add_credits_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	credits: u32,
	config: &Settings,
) -> Result<String, Error> {
	let client = Client::new();

	let password: Option<String> = None;

	let payment_intent_object = client
		.post("https://api.stripe.com/v1/payment_intents")
		.basic_auth(&config.stripe.secret_key, password)
		.form(&PaymentIntent {
			amount: credits,
			currency: "usd".to_string(),
			description: "Patr charge: Additional credits".to_string(),
			customer: config.stripe.customer_id.clone(),
		})
		.send()
		.await?
		.error_for_status()?
		.json::<PaymentIntentObject>()
		.await?;

	let metadata = json!({
		"payment_intent_id": payment_intent_object.id,
		"status": payment_intent_object.status
	});

	db::add_credits_to_workspace(
		connection,
		workspace_id,
		credits.into(),
		&metadata,
		Utc::now().into(),
	)
	.await?;

	Ok(payment_intent_object.client_secret)
}

pub async fn confirm_payment_method(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	payment_intent_id: &str,
	config: &Settings,
) -> Result<bool, Error> {
	let client = Client::new();
	let password: Option<String> = None;

	let payment_info =
		db::get_credit_info(connection, workspace_id, payment_intent_id)
			.await?
			.status(500)?;

	let payment_method_id = payment_info
		.clone()
		.metadata
		.get("payment_method_id")
		.and_then(|value| value.as_str())
		.and_then(|c| c.parse::<String>().ok())
		.status(500)?;

	if payment_intent_id != payment_method_id {
		return Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
	}

	let payment_intent_object = client
		.get(
			format!(
				"https://api.stripe.com/v1/payment_intents/{}",
				payment_intent_id
			)
			.as_str(),
		)
		.basic_auth(&config.stripe.secret_key, password)
		.send()
		.await?
		.json::<PaymentIntentObject>()
		.await?;

	let metadata = json!({
		"payment_intent_id": payment_intent_object.id,
		"status": payment_intent_object.status
	});

	db::update_workspace_credit_metadata(connection, workspace_id, &metadata)
		.await?;

	if payment_intent_object.status != Some(PaymentMethodStatus::Succeeded) &&
		payment_intent_object.amount == payment_info.credits
	{
		return Ok(false);
	}

	Ok(true)
}

/*

{
  "id": "pi_3LBGI8SFDPAh3GrI09zgntSo",
  "object": "payment_intent",
  "amount": 500,
  "amount_capturable": 0,
  "amount_details": {
	"tip": {
	}
  },
  "amount_received": 0,
  "application": null,
  "application_fee_amount": null,
  "automatic_payment_methods": null,
  "canceled_at": null,
  "cancellation_reason": null,
  "capture_method": "automatic",
  "charges": {
	"object": "list",
	"data": [

	],
	"has_more": false,
	"total_count": 0,
	"url": "/v1/charges?payment_intent=pi_3LBGI8SFDPAh3GrI09zgntSo"
  },
  "client_secret": "pi_3LBGI8SFDPAh3GrI09zgntSo_secret_mI3Ht4TFefkJu2ITNizJ4OhsC",
  "confirmation_method": "automatic",
  "created": 1655376672,
  "currency": "usd",
  "customer": null,
  "description": null,
  "invoice": null,
  "last_payment_error": null,
  "livemode": false,
  "metadata": {
  },
  "next_action": null,
  "on_behalf_of": null,
  "payment_method": null,
  "payment_method_options": {
	"card": {
	  "installments": null,
	  "mandate_options": null,
	  "network": null,
	  "request_three_d_secure": "automatic"
	}
  },
  "payment_method_types": [
	"card"
  ],
  "processing": null,
  "receipt_email": null,
  "review": null,
  "setup_future_usage": null,
  "shipping": null,
  "source": null,
  "statement_descriptor": null,
  "statement_descriptor_suffix": null,
  "status": "requires_payment_method",
  "transfer_data": null,
  "transfer_group": null
}

*/
