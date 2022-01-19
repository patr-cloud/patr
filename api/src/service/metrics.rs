use reqwest::Client;
use serde_json::json;

use crate::{
	db,
	utils::{constants::request_keys, Error},
	Database,
};

pub async fn get_deployment_metrics(
	connection: &mut <Database as sqlx::Database>::Connection,
	event: &str,
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
			"summary": format!("New activity on PATR: {}", event),
			"sections": [
				{
					"activityTitle": "PATR metrics",
					"activitySubtitle": format!("New activity on PATR: {}", event)
				},
				{
					"facts": [
						{
							"name": request_keys::USERS_TO_SIGN_UP,
							"value": sign_up_count
						},
						{
							"name": request_keys::USERS,
							"value": join_count
						},
						{
							"name": request_keys::DEPLOYMENTS,
							"value": create_deployment_count
						},
						{
							"name": request_keys::CUSTOM_DOMAINS_FOR_DEPLOYMENTS,
							"value": deployment_domain_count
						},
						{
							"name": request_keys::DELETED_DEPLOYMENTS,
							"value": deleted_deployment_count
						},
						{
							"name": request_keys::DATABASES,
							"value": create_database_count
						},
						{
							"name": request_keys::DELETED_DATABASES,
							"value": delete_database_count
						},
						{
							"name": request_keys::STATIC_SITES,
							"value": create_static_site_count
						},
						{
							"name": request_keys::CUSTOM_DOMAINS_FOR_STATIC_SITES,
							"value": static_site_domain_count
						},
						{
							"name": request_keys::DELETED_STATIC_SITES,
							"value": delete_static_site_count
						},
						{
							"name": request_keys::TOTAL_WEBSITES,
							"value": create_deployment_count + create_static_site_count
						},
						{
							"name": request_keys::TOTAL_RESOURCES,
							"value": create_deployment_count + create_database_count + create_static_site_count
						}

					]
				}
			]
		}
	);

	let _ = Client::new()
		.post("https://vicara226.webhook.office.com/webhookb2/a336cf2e-2aa4-4a33-9abb-a2c81c90b218@91758051-159a-45e9-bb70-714f7dd9de97/IncomingWebhook/9d21313b8f534ad8bcbb2447584ec657/4d71d254-6079-4d1c-8118-eb7df388e8ac")
		.json(&deployment_metrics)
		.send()
		.await;

	Ok(())
}
