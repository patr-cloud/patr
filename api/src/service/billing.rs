use api_models::utils::Uuid;
use eve_rs::AsError;
use reqwest::Client;
use serde_json::json;

use crate::{
	db,
	error,
	models::{
		AutomaticPaymentMethods,
		PaymentIntent,
		PaymentIntentObject,
		PaymentMethodStatus,
	},
	utils::{settings::Settings, Error},
	Database,
};

pub async fn add_credits_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	credits: f64,
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
			automatic_payment_methods: Some(AutomaticPaymentMethods {
				enabled: true,
			}),
		})
		.send()
		.await?
		.json::<PaymentIntentObject>()
		.await?;

	let metadata = json!({
		"payment_intent_id": payment_intent_object.id,
		"status": payment_intent_object.status
	});

	db::add_credits_to_workspace(connection, workspace_id, credits, &metadata)
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

	if payment_intent_object.status != PaymentMethodStatus::Succeeded &&
		payment_intent_object.amount == payment_info.credits
	{
		return Ok(false);
	}

	Ok(true)
}
