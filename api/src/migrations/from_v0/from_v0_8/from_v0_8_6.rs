use std::collections::BTreeMap;

use api_models::utils::Uuid;
use k8s_openapi::api::networking::v1::{
	HTTPIngressPath,
	HTTPIngressRuleValue,
	Ingress,
	IngressBackend,
	IngressRule,
	IngressServiceBackend,
	IngressSpec,
	IngressTLS,
	ServiceBackendPort,
};
use kube::{
	api::{Patch, PatchParams},
	config::{
		AuthInfo,
		Cluster,
		Context,
		Kubeconfig,
		NamedAuthInfo,
		NamedCluster,
		NamedContext,
	},
	core::ObjectMeta,
	Api,
	Config,
};
use s3::{creds::Credentials, Bucket, Region};
use sqlx::Row;

use crate::{
	migrate_query as query,
	utils::{get_current_time_millis, settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	create_static_site_deploy_history(&mut *connection, config).await?;
	add_static_site_upload_resource_type(&mut *connection, config).await?;
	add_upload_id_for_existing_users(&mut *connection, config).await?;
	rename_all_deployment_static_site_to_just_static_site(
		&mut *connection,
		config,
	)
	.await?;

	Ok(())
}

async fn create_static_site_deploy_history(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE static_site_deploy_history(
			upload_id UUID CONSTRAINT deployment_static_site_history_pk PRIMARY KEY,
			static_site_id UUID NOT NULL,
			message TEXT NOT NULL,
			created BIGINT NOT NULL
				CONSTRAINT static_site_deploy_history_chk_created_unsigned CHECK(
						created >= 0
				),
			CONSTRAINT static_site_deploy_history_fk_static_site_id
				FOREIGN KEY(static_site_id)
					REFERENCES deployment_static_site(id),
			CONSTRAINT static_site_deploy_history_uq_upload_id_static_site_id
				UNIQUE(upload_id, static_site_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}

async fn add_static_site_upload_resource_type(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	let uuid = loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				resource_type
			WHERE
				id = $1;
			"#,
			&uuid
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break uuid;
		}
	};

	query!(
		r#"
		INSERT INTO
			resource_type(
				id,
				name,
				description
			)
		VALUES
			($1, 'staticSiteUpload', NULL);
		"#,
		&uuid,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_upload_id_for_existing_users(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	let static_sites = query!(
		r#"
		SELECT
			id,
			workspace_id
		FROM
			deployment_static_site
		WHERE	
			status != 'deleted';
		"#,
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.get::<Uuid, _>("id"), row.get::<Uuid, _>("workspace_id")))
	.collect::<Vec<_>>();

	if static_sites.is_empty() {
		return Ok(());
	}

	// New resource_type for static site uploads
	let resource_type_id = query!(
		r#"
		SELECT
			id
		FROM
			resource_type
		WHERE
			name = 'staticSiteUpload';
		"#
	)
	.fetch_one(&mut *connection)
	.await
	.map(|row| row.get::<Uuid, _>("id"))?;

	// Create a new s3 bucket
	let bucket = Bucket::new(
		&config.s3.bucket,
		Region::Custom {
			endpoint: config.s3.endpoint.clone(),
			region: config.s3.region.clone(),
		},
		Credentials::new(
			Some(&config.s3.key),
			Some(&config.s3.secret),
			None,
			None,
			None,
		)
		.map_err(|_err| Error::empty())?,
	)
	.map_err(|_err| Error::empty())?;
	// Kubernetes config
	let kubernetes_config = Config::from_custom_kubeconfig(
		Kubeconfig {
			preferences: None,
			clusters: vec![NamedCluster {
				name: config.kubernetes.cluster_name.clone(),
				cluster: Cluster {
					server: config.kubernetes.cluster_url.clone(),
					insecure_skip_tls_verify: None,
					certificate_authority: None,
					certificate_authority_data: Some(
						config.kubernetes.certificate_authority_data.clone(),
					),
					proxy_url: None,
					extensions: None,
				},
			}],
			auth_infos: vec![NamedAuthInfo {
				name: config.kubernetes.auth_name.clone(),
				auth_info: AuthInfo {
					username: Some(config.kubernetes.auth_username.clone()),
					token: Some(config.kubernetes.auth_token.clone().into()),
					..Default::default()
				},
			}],
			contexts: vec![NamedContext {
				name: config.kubernetes.context_name.clone(),
				context: Context {
					cluster: config.kubernetes.cluster_name.clone(),
					user: config.kubernetes.auth_username.clone(),
					extensions: None,
					namespace: None,
				},
			}],
			current_context: Some(config.kubernetes.context_name.clone()),
			extensions: None,
			kind: Some("Config".to_string()),
			api_version: Some("v1".to_string()),
		},
		&Default::default(),
	)
	.await?;
	let kubernetes_client = kube::Client::try_from(kubernetes_config)?;
	for (static_site_id, workspace_id) in &static_sites {
		let upload_id = loop {
			let upload_id = Uuid::new_v4();

			let exists = query!(
				r#"
				SELECT
					*
				FROM
					resource
				WHERE
					id = $1;
				"#,
				&upload_id
			)
			.fetch_optional(&mut *connection)
			.await?
			.is_some();

			if !exists {
				break upload_id;
			}
		};

		// Make the existing static site as a upload resource
		let resource_name = format!("Static_site_upload: {}", upload_id);
		query!(
			r#"
			INSERT INTO
				resource(
					id,
					name
					resource_type_id,
					owner_id,
					created
				)
			VALUES
				($1, $2, $3, $4, $5);
			"#,
			&upload_id,
			&resource_name,
			&resource_type_id,
			workspace_id,
			get_current_time_millis() as i64
		)
		.execute(&mut *connection)
		.await?;

		// Make new entries in static_site_deploy_history for existing static
		// sites
		query!(
			r#"
			INSERT INTO
				static_site_deploy_history(
					upload_id,
					static_site_id,
					message,
					created,
				)
			VALUES
				($1, $2, $3);
			"#,
			&upload_id,
			static_site_id,
			get_current_time_millis() as i64
		)
		.execute(&mut *connection)
		.await?;

		// Move existing files from <static_site_id>/<file> to
		// <static_site_id>/<upload_id>/<file>

		let static_site_objects =
			bucket.list(static_site_id.to_string(), None).await?;

		for static_site in static_site_objects {
			let objects = static_site.contents;
			for object in objects {
				let (_, file) = object.key.split_once('/').unwrap();
				bucket
					.copy_object_internal(
						format!("{}/{}", static_site_id, file),
						format!("{}/{}/{}", static_site_id, upload_id, file),
					)
					.await?;
				bucket
					.delete_object(format!("{}/{}", static_site_id, file))
					.await?;
			}
		}
		let namespace = workspace_id.as_str();
		let mut annotations: BTreeMap<String, String> = BTreeMap::new();
		annotations.insert(
			"kubernetes.io/ingress.class".to_string(),
			"nginx".to_string(),
		);
		annotations.insert(
			"cert-manager.io/cluster-issuer".to_string(),
			config.kubernetes.cert_issuer_dns.clone(),
		);
		annotations.insert(
			"nginx.ingress.kubernetes.io/upstream-vhost".to_string(),
			format!("{}-{}.patr.cloud", upload_id, static_site_id),
		);
		let ingress_rule = vec![IngressRule {
			host: Some(format!("{}-{}.patr.cloud", upload_id, static_site_id)),
			http: Some(HTTPIngressRuleValue {
				paths: vec![HTTPIngressPath {
					backend: IngressBackend {
						service: Some(IngressServiceBackend {
							name: format!("service-{}", static_site_id),
							port: Some(ServiceBackendPort {
								number: Some(80),
								..ServiceBackendPort::default()
							}),
						}),
						..Default::default()
					},
					path: Some("/".to_string()),
					path_type: Some("Prefix".to_string()),
				}],
			}),
		}];
		let patr_domain_tls = vec![IngressTLS {
			hosts: Some(vec![
				"*.patr.cloud".to_string(),
				"patr.cloud".to_string(),
			]),
			secret_name: None,
		}];
		let kubernetes_ingress = Ingress {
			metadata: ObjectMeta {
				name: Some(format!("ingress-{}", static_site_id)),
				annotations: Some(annotations),
				..ObjectMeta::default()
			},
			spec: Some(IngressSpec {
				rules: Some(ingress_rule),
				tls: Some(patr_domain_tls),
				..IngressSpec::default()
			}),
			..Ingress::default()
		};
		let ingress_api: Api<Ingress> =
			Api::namespaced(kubernetes_client.clone(), namespace);
		ingress_api
			.patch(
				&format!("ingress-{}", static_site_id),
				&PatchParams::apply(&format!("ingress-{}", static_site_id)),
				&Patch::Apply(kubernetes_ingress),
			)
			.await?;
	}

	Ok(())
}

async fn rename_all_deployment_static_site_to_just_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE deployment_static_site
		RENAME TO static_site;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE static_site
		RENAME CONSTRAINT deployment_static_site_chk_name_is_trimmed
		TO static_site_chk_name_is_trimmed;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE static_site
		RENAME CONSTRAINT deployment_static_site_uq_name_workspace_id
		TO static_site_uq_name_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE static_site
		RENAME CONSTRAINT deployment_static_site_uq_id_workspace_id
		TO static_site_uq_id_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE static_site
		RENAME CONSTRAINT deployment_static_site_fk_id_workspace_id
		TO static_site_fk_id_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
