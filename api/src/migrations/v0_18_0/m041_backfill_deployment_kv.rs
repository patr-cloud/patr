//! Rewrites every live deployment's ingress KV entry from the database, in the
//! shape that carries `should_run` instead of a status. The worker routes on
//! `should_run`, and entries written before it existed don't have it.
//!
//! - Cloud only. Self-hosted has no ingress KV, so there it does nothing.
//! - Each entry is rebuilt whole from the database, so running it again writes the same values, and
//!   a failed write rolls the migration back to run again on the next start.

#[cfg(feature = "cloud")]
use std::collections::BTreeMap;

#[cfg(feature = "cloud")]
use cloudflare::{
	endpoints::workerskv::write_key,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
#[cfg(feature = "cloud")]
use models::cloudflare::kv::InternalKVData;

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection, config: &AppConfig) -> Result<(), ErrorType> {
	cfg_if! {
		if #[cfg(feature = "cloud")] {
			let deployments = sqlx::query_as::<_, (Uuid, Uuid, bool)>(
				r#"
				SELECT
					id,
					runner,
					should_run
				FROM
					deployment
				WHERE
					deleted IS NULL;
				"#,
			)
			.fetch_all(&mut *connection)
			.await?;

			let mut ports = BTreeMap::<Uuid, Vec<u16>>::new();
			for (deployment_id, port) in sqlx::query_as::<_, (Uuid, i32)>(
				r#"
				SELECT
					deployment_id,
					port
				FROM
					deployment_exposed_port
				ORDER BY
					deployment_id,
					port;
				"#,
			)
			.fetch_all(&mut *connection)
			.await?
			{
				ports.entry(deployment_id).or_default().push(port as u16);
			}

			info!(
				"Rewriting the ingress KV entries of {} deployments",
				deployments.len()
			);

			let client = CloudflareClient::new(
				Credentials::UserAuthToken {
					token: config.cloudflare.api_key.clone(),
				},
				ClientConfig::default(),
				Environment::Custom(config.cloudflare.base_url.clone()),
			)?;
			for (deployment_id, runner_id, should_run) in deployments {
				client
					.request(&write_key::WriteKey {
						account_identifier: &config.cloudflare.account_id,
						namespace_identifier: &config.cloudflare.worker_namespace_id,
						key: &deployment_id.to_string(),
						params: write_key::WriteKeyParams {
							expiration: None,
							expiration_ttl: None,
						},
						body: write_key::WriteKeyBody::Value(serde_json::to_vec(
							&InternalKVData::Deployment {
								ports: ports.remove(&deployment_id).unwrap_or_default(),
								runner_id,
								should_run,
							},
						)?),
					})
					.await?;
			}
		} else {
			let _ = (connection, config);
		}
	}

	Ok(())
}
