use api_models::utils::DateTime;
use chrono::Utc;
use reqwest::Client;
use tokio::time;

use crate::{
	db,
	models::{rabbitmq::MigrationChangeData, IpQualityScore},
	service,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn process_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	request_data: MigrationChangeData,
	config: &Settings,
) -> Result<(), Error> {
	match request_data {
		MigrationChangeData::CheckUserAccountForSpam {
			user_id,
			process_after: DateTime(process_after),
			request_id,
		} => {
			if Utc::now() < process_after {
				// process_after is in the future. Wait for a while and requeue
				time::sleep(time::Duration::from_millis(
					if cfg!(debug_assertions) { 1000 } else { 60_000 },
				))
				.await;
				return Err(Error::empty());
			}

			// Can process without being affected by rate limit

			let emails =
				db::get_personal_emails_for_user(connection, &user_id).await?;

			let mut is_user_spam = false;
			let mut is_email_disposable = false;

			for email in emails {
				// Check if any one of their emails are spam or disposable
				let spam_score = Client::new()
					.get(format!(
						"{}/{}/{}",
						config.ip_quality.host, config.ip_quality.token, email
					))
					.send()
					.await?
					.json::<IpQualityScore>()
					.await?;

				if spam_score.disposable || spam_score.fraud_score > 75 {
					is_user_spam = spam_score.fraud_score > 75;
					is_email_disposable = spam_score.disposable;
					break;
				}
			}

			if !is_user_spam && !is_email_disposable {
				log::info!(
					"User ID {} is neither spam nor disposable. Ignoring...",
					user_id
				);
				return Ok(());
			}

			let workspaces =
				db::get_all_workspaces_for_user(connection, &user_id).await?;
			let workspaces_len = workspaces.len();
			for (index, workspace) in workspaces.into_iter().enumerate() {
				log::info!(
					"Checking workspace {}/{} for user {}",
					index + 1,
					workspaces_len,
					user_id
				);
				let deployments = db::get_deployments_for_workspace(
					connection,
					&workspace.id,
				)
				.await?;

				// Delete all the deployments for that workspace
				// In case it's a disposable email, delete from DB as well as
				// k8s. For spam accounts, only delete from k8s.

				let deployments_num = deployments.len();
				log::info!(
					"Found {} deployments for workspace {}. Deleting...",
					deployments_num,
					workspace.id
				);

				for (index, deployment) in deployments.into_iter().enumerate() {
					log::info!(
						"Deleting deployment {}/{} for workspace {}",
						index + 1,
						deployments_num,
						workspace.id
					);
					if is_user_spam {
						let kubeconfig =
							service::get_kubernetes_config_for_region(
								connection,
								&deployment.region,
								config,
							)
							.await?;
						service::delete_kubernetes_deployment(
							&workspace.id,
							&deployment.id,
							kubeconfig,
							&request_id,
						)
						.await?;
					} else {
						service::delete_deployment(
							connection,
							&workspace.id,
							&deployment.id,
							&deployment.region,
							None,
							None,
							"0.0.0.0",
							true,
							config,
							&request_id,
						)
						.await?;
					}
				}
				if is_email_disposable {
					log::info!(
                        "Workspace {} has a disposable email. Marking limits to 0",
                        workspace.id
                    );
					// Set their workspace limits to 0
					db::set_resource_limit_for_workspace(
						connection,
						&workspace.id,
						0,
						0,
						0,
						0,
						0,
						0,
						0,
					)
					.await?;
				}

				if is_user_spam {
					log::info!(
                        "Workspace {} has a high spam rating email. Marking as spam",
                        workspace.id
                    );
					// Mark their workspace as spam
					db::mark_workspace_as_spam(connection, &workspace.id)
						.await?;
				}
			}

			Ok(())
		}
	}
}
