use std::{fmt::Debug, net::IpAddr};

use api_models::utils::Uuid;
use either::Either;
use k8s_openapi::api::{
	apps::v1::Deployment,
	autoscaling::v1::HorizontalPodAutoscaler,
	core::v1::{ConfigMap, Service},
	networking::v1::Ingress,
};
use kube::{
	api::DeleteParams,
	client::Status,
	config::{
		AuthInfo,
		Cluster,
		Context,
		Kubeconfig,
		NamedAuthInfo,
		NamedCluster,
		NamedContext,
	},
	error::ErrorResponse,
	Api,
	Config,
	Error as KubeError,
	Result,
};
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sqlx::Row;

use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct IpQualityScore {
	pub valid: bool,
	pub disposable: bool,
	pub fraud_score: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterType {
	PatrOwned,
	UserOwned {
		region_id: Uuid,
		ingress_ip_addr: IpAddr,
	},
}

#[derive(Debug, Clone)]
pub struct KubernetesAuthDetails {
	pub cluster_url: String,
	pub auth_username: String,
	pub auth_token: String,
	pub certificate_authority_data: String,
}

#[derive(Debug, Clone)]
pub struct KubernetesConfigDetails {
	pub cluster_type: ClusterType,
	pub auth_details: KubernetesAuthDetails,
}

#[async_trait::async_trait]
pub trait DeleteOpt<T> {
	async fn delete_opt(
		&self,
		name: &str,
		dp: &DeleteParams,
	) -> Result<Option<Either<T, Status>>>;
}

#[async_trait::async_trait]
impl<T> DeleteOpt<T> for Api<T>
where
	T: Clone + DeserializeOwned + Debug,
{
	async fn delete_opt(
		&self,
		name: &str,
		dp: &DeleteParams,
	) -> Result<Option<Either<T, Status>>> {
		match self.delete(name, dp).await {
			Ok(obj) => Ok(Some(obj)),
			Err(KubeError::Api(ErrorResponse { code: 404, .. })) => Ok(None),
			Err(err) => Err(err),
		}
	}
}

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	mark_existing_workspaces_as_spam(connection, config).await?;
	Ok(())
}

pub(super) async fn mark_existing_workspaces_as_spam(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	let users = query!(
		r#"
		SELECT
			id
		FROM
			"user";
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| row.get::<Uuid, _>("id"))
	.collect::<Vec<_>>();

	let total_users = users.len();

	for (idx, user_id) in users.into_iter().enumerate() {
		log::trace!(
			"request_id: {} migrating user - {}/{} with user_id: {}",
			request_id,
			idx + 1,
			total_users,
			user_id
		);
		let emails = query!(
			r#"
			SELECT
				CONCAT(
					personal_email.local,
					'@',
					domain.name,
					'.',
					domain.tld
				) as "email"
			FROM
				personal_email
			INNER JOIN
				domain
			ON
				personal_email.domain_id = domain.id
			WHERE
				personal_email.user_id = $1;
			"#,
			&user_id,
		)
		.fetch_all(&mut *connection)
		.await?
		.into_iter()
		.map(|row| row.get::<String, _>("email"))
		.collect::<Vec<_>>();

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
				log::info!(
					"User ID {} with email: {} is a {} email",
					user_id,
					email,
					if spam_score.disposable {
						"disposable"
					} else {
						"spam"
					}
				);
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

			continue;
		}

		let workspaces = query!(
			r#"
			SELECT DISTINCT
				workspace.id
			FROM
				workspace
			LEFT JOIN
				workspace_user
			ON
				workspace.id = workspace_user.workspace_id
			WHERE
				(
					workspace.super_admin_id = $1 OR
					workspace_user.user_id = $1
				) AND
				workspace.deleted IS NULL;
			"#,
			&user_id,
		)
		.fetch_all(&mut *connection)
		.await?
		.into_iter()
		.map(|row| row.get::<Uuid, _>("id"))
		.collect::<Vec<_>>();

		let workspaces_len = workspaces.len();
		for (index, workspace_id) in workspaces.into_iter().enumerate() {
			log::info!(
				"Checking workspace {}/{} for user {}",
				index + 1,
				workspaces_len,
				user_id
			);
			let deployments = query!(
				r#"
				SELECT
					id,
					region
				FROM
					deployment
				WHERE
					workspace_id = $1 AND
					status != 'deleted';
				"#,
				&workspace_id
			)
			.fetch_all(&mut *connection)
			.await?
			.into_iter()
			.map(|row| (row.get::<Uuid, _>("id"), row.get::<Uuid, _>("region")))
			.collect::<Vec<_>>();

			let deployments_num = deployments.len();
			log::info!(
				"Found {} deployments for workspace {}. Deleting...",
				deployments_num,
				&workspace_id
			);

			for (index, (deployment_id, deployment_region)) in
				deployments.into_iter().enumerate()
			{
				log::info!(
					"Deleting deployment {}/{} for workspace {}",
					index + 1,
					deployments_num,
					&workspace_id
				);

				let (
					ready,
					region_workspace_id,
					kubernetes_cluster_url,
					kubernetes_auth_username,
					kubernetes_auth_token,
					kubernetes_ca_data,
					kubernetes_ingress_ip_addr,
				) = query!(
					r#"
					SELECT
						*
					FROM
						deployment_region
					WHERE
						id = $1;
					"#,
					&deployment_region,
				)
				.fetch_one(&mut *connection)
				.await
				.map(|row| {
					(
						row.get::<bool, _>("ready"),
						row.get::<Option<Uuid>, _>("workspace_id"),
						row.get::<Option<String>, _>("kubernetes_cluster_url"),
						row.get::<Option<String>, _>(
							"kubernetes_auth_username",
						),
						row.get::<Option<String>, _>("kubernetes_auth_token"),
						row.get::<Option<String>, _>("kubernetes_ca_data"),
						row.get::<Option<IpAddr>, _>(
							"kubernetes_ingress_ip_addr",
						),
					)
				})?;

				let kubeconfig = get_kubernetes_config_for_region(
					deployment_region,
					ready,
					region_workspace_id.clone(),
					kubernetes_cluster_url,
					kubernetes_auth_username,
					kubernetes_auth_token,
					kubernetes_ca_data,
					kubernetes_ingress_ip_addr,
					config,
				)
				.await?;

				if is_user_spam {
					log::info!(
					"Workspace {} has a high spam rating email. Marking as spam",
					workspace_id);

					delete_deployment_from_kubernetes(
						&workspace_id,
						&deployment_id,
						kubeconfig,
						&request_id,
					)
					.await?;

					// Mark their workspace as spam
					query!(
						r#"
						UPDATE
							workspace
						SET
							is_spam = TRUE
						WHERE
							id = $1;
						"#,
						&workspace_id,
					)
					.execute(&mut *connection)
					.await?;
				} else {
					delete_deployment(
						connection,
						&deployment_id,
						&workspace_id,
						region_workspace_id.as_ref(),
						kubeconfig,
						&request_id,
					)
					.await?;

					log::info!(
						"Workspace {} has a disposable email. Marking limits to 0",
						workspace_id
					);

					// Set their workspace limits to 0
					query!(
						r#"
						UPDATE
							workspace
						SET
							deployment_limit = 0,
							database_limit = 0,
							static_site_limit = 0,
							managed_url_limit = 0,
							docker_repository_storage_limit = 0,
							domain_limit = 0,
							secret_limit = 0
						WHERE
							id = $1;
						"#,
						&workspace_id,
					)
					.execute(&mut *connection)
					.await?;
				}
			}
		}
	}
	Ok(())
}

pub(super) async fn get_kubernetes_config_for_region(
	region_id: Uuid,
	ready: bool,
	workspace_id: Option<Uuid>,
	kubernetes_cluster_url: Option<String>,
	kubernetes_auth_username: Option<String>,
	kubernetes_auth_token: Option<String>,
	kubernetes_ca_data: Option<String>,
	kubernetes_ingress_ip_addr: Option<IpAddr>,
	config: &Settings,
) -> Result<KubernetesConfigDetails, Error> {
	let kubeconfig = if workspace_id.is_none() {
		get_kubernetes_config_for_default_region(config)
	} else {
		match (
			ready,
			kubernetes_cluster_url,
			kubernetes_auth_username,
			kubernetes_auth_token,
			kubernetes_ca_data,
			kubernetes_ingress_ip_addr,
		) {
			(
				true,
				Some(cluster_url),
				Some(auth_username),
				Some(auth_token),
				Some(certificate_authority_data),
				Some(ingress_ip_addr),
			) => KubernetesConfigDetails {
				cluster_type: ClusterType::UserOwned {
					region_id,
					ingress_ip_addr,
				},
				auth_details: KubernetesAuthDetails {
					cluster_url,
					auth_username,
					auth_token,
					certificate_authority_data,
				},
			},
			_ => {
				log::info!("cluster {region_id} is not yet initialized");
				return Err(Error::empty().body(format!(
					"cluster {region_id} is not yet initialized"
				)));
			}
		}
	};
	Ok(kubeconfig)
}

pub(super) fn get_kubernetes_config_for_default_region(
	config: &Settings,
) -> KubernetesConfigDetails {
	KubernetesConfigDetails {
		cluster_type: ClusterType::PatrOwned,
		auth_details: KubernetesAuthDetails {
			cluster_url: config.kubernetes.cluster_url.to_owned(),
			auth_username: config.kubernetes.auth_username.to_owned(),
			auth_token: config.kubernetes.auth_token.to_owned(),
			certificate_authority_data: config
				.kubernetes
				.certificate_authority_data
				.to_owned(),
		},
	}
}

pub(super) async fn delete_deployment_from_kubernetes(
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	kubeconfig: KubernetesConfigDetails,
	request_id: &Uuid,
) -> Result<(), Error> {
	let kubernetes_client =
		get_kubernetes_client(kubeconfig.auth_details).await?;

	let namespace = workspace_id.to_string();
	log::trace!("request_id: {} - deleting the deployment", request_id);

	Api::<Deployment>::namespaced(kubernetes_client.clone(), &namespace)
		.delete_opt(
			&format!("deployment-{}", deployment_id),
			&DeleteParams::default(),
		)
		.await?;

	log::trace!("request_id: {} - deleting the config map", request_id);

	Api::<ConfigMap>::namespaced(kubernetes_client.clone(), &namespace)
		.delete_opt(
			&format!("config-mount-{}", deployment_id),
			&DeleteParams::default(),
		)
		.await?;

	log::trace!("request_id: {} - deleting the service", request_id);
	Api::<Service>::namespaced(kubernetes_client.clone(), &namespace)
		.delete_opt(
			&format!("service-{}", deployment_id),
			&DeleteParams::default(),
		)
		.await?;

	log::trace!("request_id: {} - deleting the hpa", request_id);

	Api::<HorizontalPodAutoscaler>::namespaced(
		kubernetes_client.clone(),
		&namespace,
	)
	.delete_opt(&format!("hpa-{}", deployment_id), &DeleteParams::default())
	.await?;

	log::trace!("request_id: {} - deleting the ingress", request_id);
	Api::<Ingress>::namespaced(kubernetes_client, &namespace)
		.delete_opt(
			&format!("ingress-{}", deployment_id),
			&DeleteParams::default(),
		)
		.await?;

	log::trace!(
		"request_id: {} - deployment deleted successfully!",
		request_id
	);

	Ok(())
}

async fn get_kubernetes_client(
	kube_auth_details: KubernetesAuthDetails,
) -> Result<kube::Client, Error> {
	let kubeconfig = Config::from_custom_kubeconfig(
		Kubeconfig {
			api_version: Some("v1".to_string()),
			kind: Some("Config".to_string()),
			clusters: vec![NamedCluster {
				name: "kubernetesCluster".to_owned(),
				cluster: Cluster {
					server: kube_auth_details.cluster_url,
					certificate_authority_data: Some(
						kube_auth_details.certificate_authority_data,
					),
					insecure_skip_tls_verify: None,
					certificate_authority: None,
					proxy_url: None,
					extensions: None,
				},
			}],
			auth_infos: vec![NamedAuthInfo {
				name: kube_auth_details.auth_username.clone(),
				auth_info: AuthInfo {
					token: Some(kube_auth_details.auth_token.into()),
					..Default::default()
				},
			}],
			contexts: vec![NamedContext {
				name: "kubernetesContext".to_owned(),
				context: Context {
					cluster: "kubernetesCluster".to_owned(),
					user: kube_auth_details.auth_username,
					extensions: None,
					namespace: None,
				},
			}],
			current_context: Some("kubernetesContext".to_owned()),
			preferences: None,
			extensions: None,
		},
		&Default::default(),
	)
	.await?;

	let kube_client = kube::Client::try_from(kubeconfig)?;
	Ok(kube_client)
}

pub(super) async fn delete_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	deployment_workspace_id: &Uuid,
	region_workspace_id: Option<&Uuid>,
	kubeconfig: KubernetesConfigDetails,
	request_id: &Uuid,
) -> Result<(), Error> {
	if region_workspace_id.is_none() {
		query!(
			r#"
			UPDATE
				deployment_payment_history
			SET
				stop_time = NOW()
			WHERE
				deployment_id = $1 AND
				stop_time IS NULL;
			"#,
			deployment_id
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		UPDATE
			deployment
		SET
			status = 'deleted',
			deleted = COALESCE(
				deleted,
				NOW()
			)
		WHERE
			id = $1;
		"#,
		deployment_id
	)
	.execute(&mut *connection)
	.await?;

	delete_deployment_from_kubernetes(
		deployment_workspace_id,
		deployment_id,
		kubeconfig,
		request_id,
	)
	.await?;

	Ok(())
}
