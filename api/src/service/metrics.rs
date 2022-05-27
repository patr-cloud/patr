use reqwest::Client;
use serde_json::json;

use crate::{
	db,
	utils::{constants::request_keys, settings::Settings, Error},
	Database,
};

pub async fn get_internal_metrics(
	connection: &mut <Database as sqlx::Database>::Connection,
	_: &str,
) -> Result<(), Error> {
	// Do not send deployment metrics for debug builds
	if cfg!(debug_assertions) {
		return Ok(());
	}

	let sign_up_count = db::get_sign_up_count(connection).await?;
	let join_count = db::get_join_count(connection).await?;
	let create_deployment_count =
		db::get_created_deployment_count(connection).await?;
	let deployment_domain_count =
		db::get_deployment_domain_count(connection).await?;
	let deleted_deployment_count =
		db::get_deleted_deployment_count(connection).await?;
	let create_database_count =
		db::get_created_database_count(connection).await?;
	let delete_database_count =
		db::get_deleted_database_count(connection).await?;
	let create_static_site_count =
		db::get_created_static_site_count(connection).await?;
	let static_site_domain_count =
		db::get_static_site_domain_count(connection).await?;
	let delete_static_site_count =
		db::get_deleted_static_site_count(connection).await?;

	let _ = Client::new()
		.post("https://api-eu.mixpanel.com/track")
		.json(&json!([
			{
				"properties": {
					"token": "9603636038d7943b22901e39a35ea08b",
					"unsuccessfulSignUpCount": sign_up_count,
					"successfulSignUpCount": join_count,
					"undeletedDeploymentsCount": create_deployment_count,
					"deletedDeploymentsCount": deleted_deployment_count,
					"deploymentWithDomainsCount": deployment_domain_count,
					"undeletedDatabasesCount": create_database_count,
					"deletedDatabasesCount": delete_database_count,
					"undeletedStaticSitesCount": create_static_site_count,
					"deletedStaticSitesCount": delete_static_site_count,
					"staticSiteWithDomainsCount": static_site_domain_count,
				},
				"event": "Metrics"
			}
		]))
		.send()
		.await;

	Ok(())
}

pub async fn include_user_to_mailchimp(
	connection: &mut <Database as sqlx::Database>::Connection,
	email: &str,
	first_name: &str,
	last_name: &str,
	config: &Settings,
) -> Result<(), Error> {
	// Do not send deployment metrics for debug builds
	if cfg!(debug_assertions) {
		return Ok(());
	}

	let sign_up_count = db::get_sign_up_count(connection).await?;
	let join_count = db::get_join_count(connection).await?;
	let create_deployment_count =
		db::get_created_deployment_count(connection).await?;
	let deployment_domain_count =
		db::get_deployment_domain_count(connection).await?;
	let deleted_deployment_count =
		db::get_deleted_deployment_count(connection).await?;
	let create_database_count =
		db::get_created_database_count(connection).await?;
	let delete_database_count =
		db::get_deleted_database_count(connection).await?;
	let create_static_site_count =
		db::get_created_static_site_count(connection).await?;
	let static_site_domain_count =
		db::get_static_site_domain_count(connection).await?;
	let delete_static_site_count =
		db::get_deleted_static_site_count(connection).await?;

	let deployment_metrics = json!(
		{
			"@type": "MessageCard",
			"@context": "http://schema.org/extensions",
			"themeColor": "0076D7",
			"contentType": "application/vnd.microsoft.teams.card.o365connector",
			"summary": "New sign up",
			"sections": [
				{
					"activityTitle": "PATR metrics",
					"activitySubtitle": "New sign up"
				},
				{
					"facts": [
						{
							"name": "who the fuck signed up",
							"value": format!("{} {} ({})", first_name, last_name, email)
						},
						{
							"name": "users to sign up",
							"value": sign_up_count
						},
						{
							"name": "users",
							"value": join_count
						},
						{
							"name": "deployments",
							"value": create_deployment_count
						},
						{
							"name": "custom domains for deployments",
							"value": deployment_domain_count
						},
						{
							"name": "deleted deployments",
							"value": deleted_deployment_count
						},
						{
							"name": request_keys::DATABASES,
							"value": create_database_count
						},
						{
							"name": "deleted databases",
							"value": delete_database_count
						},
						{
							"name": "static sites",
							"value": create_static_site_count
						},
						{
							"name": "custom domains for static sites",
							"value": static_site_domain_count
						},
						{
							"name": "deleted static sites",
							"value": delete_static_site_count
						},
						{
							"name": "total websites",
							"value": create_deployment_count + create_static_site_count
						},
						{
							"name": "total resources",
							"value": create_deployment_count + create_database_count + create_static_site_count
						}
					]
				}
			]
		}
	);

	let _ = Client::new()
		.put(format!(
			"https://us20.api.mailchimp.com/3.0/lists/{}/members/{}",
			config.mailchimp.list_id, email
		))
		.basic_auth("anystring", Some(config.mailchimp.api_key.clone()))
		.json(&json!({
			"email_address": email,
			"status": "subscribed",
			"tags": ["patr-app-user"],
			"merge_fields": {
				"FNAME": first_name,
				"LNAME": last_name
			}
		}))
		.send()
		.await;

	let _ = Client::new()
		.post(format!(
			"https://{}/webhookb2/{}@{}/IncomingWebhook/{}/{}",
			"vicara226.webhook.office.com",
			"a336cf2e-2aa4-4a33-9abb-a2c81c90b218",
			"91758051-159a-45e9-bb70-714f7dd9de97",
			"1cafe2443c294bb59383729eded825c5",
			"4d71d254-6079-4d1c-8118-eb7df388e8ac",
		))
		.json(&deployment_metrics)
		.send()
		.await;

	Ok(())
}
