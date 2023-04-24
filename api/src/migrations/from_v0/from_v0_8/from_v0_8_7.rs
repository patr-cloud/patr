use std::collections::BTreeMap;

use api_models::utils::Uuid;
use chrono::Utc;
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
	api::{DeleteParams, Patch, PatchParams},
	config::{
		AuthInfo,
		Cluster,
		Context,
		Kubeconfig,
		NamedAuthInfo,
		NamedCluster,
		NamedContext,
	},
	core::{DynamicObject, ObjectMeta},
	discovery::ApiResource,
	Api,
	Config,
};
use s3::{creds::Credentials, Bucket, Region};
use sqlx::Row;

use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	create_static_site_upload_history(&mut *connection, config).await?;
	add_static_site_upload_resource_type(&mut *connection, config).await?;
	add_upload_id_for_existing_users(&mut *connection, config).await?;
	rename_all_deployment_static_site_to_just_static_site(
		&mut *connection,
		config,
	)
	.await?;
	add_last_unverified_column_to_workspace_domain(connection, config).await?;
	add_table_deployment_image_digest(&mut *connection, config).await?;
	populate_deployment_deploy_history(&mut *connection, config).await?;
	create_deployment_config_file(&mut *connection, config).await?;
	update_dns_record_name_constraint_regexp(&mut *connection, config).await?;
	add_is_configured_for_managed_urls(&mut *connection, config).await?;
	fix_july_billing_issues(&mut *connection, config).await?;

	Ok(())
}

async fn create_static_site_upload_history(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE static_site_upload_history(
			upload_id UUID
				CONSTRAINT static_site_upload_history_pk PRIMARY KEY
				CONSTRAINT static_site_upload_history_fk_upload_id_resource_id
					REFERENCES resource(id),
			static_site_id UUID NOT NULL CONSTRAINT
				static_site_upload_history_fk_static_site_id
					REFERENCES deployment_static_site(id),
			message TEXT NOT NULL,
			uploaded_by UUID NOT NULL CONSTRAINT
				static_site_upload_history_fk_uploaded_by
					REFERENCES "user"(id),
			created TIMESTAMPTZ NOT NULL,
			CONSTRAINT static_site_upload_history_uq_upload_id_static_site_id
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
	let resource_type_id = loop {
		let resource_type_id = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				resource_type
			WHERE
				id = $1;
			"#,
			&resource_type_id
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break resource_type_id;
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
			($1, 'staticSiteUpload', '');
		"#,
		&resource_type_id
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_upload_id_for_existing_users(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE deployment_static_site
		ADD COLUMN current_live_upload UUID;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_site
		ADD CONSTRAINT static_site_fk_current_live_upload
		FOREIGN KEY(id, current_live_upload) REFERENCES
		static_site_upload_history(static_site_id, upload_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	let static_sites = query!(
		r#"
		SELECT
			deployment_static_site.id,
			workspace_id,
			created
		FROM
			deployment_static_site
		INNER JOIN
			resource
		ON
			deployment_static_site.id = resource.id
		WHERE
			status != 'deleted' AND
			status != 'created';
		"#,
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| {
		(
			row.get::<Uuid, _>("id"),
			row.get::<Uuid, _>("workspace_id"),
			row.get::<i64, _>("created"),
		)
	})
	.collect::<Vec<_>>();
	let sites_len = static_sites.len();

	let static_site_upload_resource_type_id = query!(
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
	.await?
	.get::<Uuid, _>("id");

	if static_sites.is_empty() {
		return Ok(());
	}

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
				cluster: Some(Cluster {
					server: Some(config.kubernetes.cluster_url.clone()),
					insecure_skip_tls_verify: None,
					certificate_authority: None,
					certificate_authority_data: Some(
						config.kubernetes.certificate_authority_data.clone(),
					),
					proxy_url: None,
					extensions: None,
					..Default::default()
				}),
			}],
			auth_infos: vec![NamedAuthInfo {
				name: config.kubernetes.auth_name.clone(),
				auth_info: Some(AuthInfo {
					username: Some(config.kubernetes.auth_username.clone()),
					token: Some(config.kubernetes.auth_token.clone().into()),
					..Default::default()
				}),
			}],
			contexts: vec![NamedContext {
				name: config.kubernetes.context_name.clone(),
				context: Some(Context {
					cluster: config.kubernetes.cluster_name.clone(),
					user: config.kubernetes.auth_username.clone(),
					extensions: None,
					namespace: None,
				}),
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
	for (index, (static_site_id, workspace_id, created)) in
		static_sites.into_iter().enumerate()
	{
		log::trace!("Updating static site {}/{}", index, sites_len);
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

		let super_admin_id = query!(
			r#"
			SELECT
				super_admin_id
			FROM
				workspace
			WHERE
				id = $1
			"#,
			&workspace_id
		)
		.fetch_one(&mut *connection)
		.await?
		.get::<Uuid, _>("super_admin_id");

		// Make new entries in static_site_upload_history for existing static
		// sites
		query!(
			r#"
			INSERT INTO
				resource(
					id,
					name,
					resource_type_id,
					owner_id,
					created
				)
			VALUES
				($1, $2, $3, $4, $5);
			"#,
			&upload_id,
			format!("Static site upload: {}", upload_id),
			&static_site_upload_resource_type_id,
			&workspace_id,
			created,
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			INSERT INTO
				static_site_upload_history(
					upload_id,
					static_site_id,
					message,
					uploaded_by,
					created
				)
			VALUES
				($1, $2, 'No upload message', $3, $4);
			"#,
			&upload_id,
			&static_site_id,
			&super_admin_id,
			&Utc::now(),
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			UPDATE
				deployment_static_site
			SET
				current_live_upload = $1
			WHERE
				id = $2;
			"#,
			&upload_id,
			&static_site_id,
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
				let file = if let Some(removed) =
					object.key.strip_prefix(&format!("{}/", static_site_id))
				{
					removed
				} else {
					continue;
				};
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
					path_type: "Prefix".to_string(),
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

	let managed_urls = query!(
		r#"
		SELECT
			managed_url.id,
			deployment_static_site.current_live_upload,
			managed_url.static_site_id,
			managed_url.workspace_id,
			managed_url.sub_domain,
			CONCAT(domain.name, '.', domain.tld) as "domain",
			managed_url.path,
			workspace_domain.nameserver_type::TEXT
		FROM
			managed_url
		INNER JOIN
			deployment_static_site
		ON
			deployment_static_site.id = managed_url.static_site_id
		INNER JOIN
			workspace_domain
		ON
			workspace_domain.id = managed_url.domain_id
		INNER JOIN
			domain
		ON
			domain.id = workspace_domain.id
		WHERE
			managed_url.sub_domain NOT LIKE 'patr-deleted: %' AND
			url_type = 'proxy_to_static_site' AND
			workspace_domain.is_verified = TRUE;
		"#,
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| {
		(
			row.get::<Uuid, _>("id"),
			row.get::<Option<Uuid>, _>("current_live_upload"),
			row.get::<Uuid, _>("static_site_id"),
			row.get::<Uuid, _>("workspace_id"),
			row.get::<String, _>("sub_domain"),
			row.get::<String, _>("domain"),
			row.get::<String, _>("path"),
			row.get::<String, _>("nameserver_type"),
		)
	});

	for (
		id,
		current_live_upload,
		static_site_id,
		workspace_id,
		sub_domain,
		domain,
		path,
		nameserver_type,
	) in managed_urls
	{
		let namespace = workspace_id.as_str();

		let annotations: BTreeMap<String, String> = [
			(
				"kubernetes.io/ingress.class".to_string(),
				"nginx".to_string(),
			),
			(
				"nginx.ingress.kubernetes.io/upstream-vhost".to_string(),
				if let Some(upload_id) = current_live_upload {
					format!("{}-{}.patr.cloud", upload_id, static_site_id)
				} else {
					format!("{}.patr.cloud", static_site_id)
				},
			),
			(
				"cert-manager.io/cluster-issuer".to_string(),
				if nameserver_type == "internal" {
					config.kubernetes.cert_issuer_dns.clone()
				} else {
					config.kubernetes.cert_issuer_http.clone()
				},
			),
		]
		.into_iter()
		.collect();
		let ingress_rule = vec![IngressRule {
			host: Some(format!("{}.{}", sub_domain, domain)),
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
					path: Some(path),
					path_type: "Prefix".to_string(),
				}],
			}),
		}];
		let kubernetes_ingress = Ingress {
			metadata: ObjectMeta {
				name: Some(format!("ingress-{}", id)),
				annotations: Some(annotations),
				..ObjectMeta::default()
			},
			spec: Some(IngressSpec {
				rules: Some(ingress_rule),
				tls: None,
				..IngressSpec::default()
			}),
			..Ingress::default()
		};
		Api::<Ingress>::namespaced(kubernetes_client.clone(), namespace)
			.patch(
				&format!("ingress-{}", id),
				&PatchParams::apply(&format!("ingress-{}", id)),
				&Patch::Apply(kubernetes_ingress),
			)
			.await?
			.status
			.status(500)?;
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
		RENAME CONSTRAINT deployment_static_site_pk
		TO static_site_pk;
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

async fn add_last_unverified_column_to_workspace_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE workspace_domain
		ADD COLUMN last_unverified TIMESTAMPTZ NOT NULL
		DEFAULT NOW();
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Remove default value
	query!(
		r#"
		ALTER TABLE workspace_domain
		ALTER COLUMN last_unverified DROP DEFAULT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_table_deployment_image_digest(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE deployment_deploy_history(
			deployment_id UUID NOT NULL
				CONSTRAINT deployment_image_digest_fk_deployment_id
					REFERENCES deployment(id),
			image_digest TEXT NOT NULL,
			repository_id UUID NOT NULL
				CONSTRAINT deployment_image_digest_fk_repository_id
					REFERENCES docker_registry_repository(id),
			created TIMESTAMPTZ NOT NULL,
			CONSTRAINT deployment_image_digest_pk
				PRIMARY KEY(deployment_id, image_digest)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		ADD COLUMN current_live_digest TEXT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		ADD CONSTRAINT deployment_fk_current_live_digest
		FOREIGN KEY(id, current_live_digest) REFERENCES
		deployment_deploy_history(deployment_id, image_digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn populate_deployment_deploy_history(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	let deployments = query!(
		r#"
		SELECT
			id,
			repository_id,
			image_tag
		FROM
			deployment
		WHERE
			status != 'deleted' AND
			status != 'created';
		"#,
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| {
		(
			row.get::<Uuid, _>("id"),
			row.get::<Uuid, _>("repository_id"),
			row.get::<String, _>("image_tag"),
		)
	});

	for (deployment_id, repository_id, image_tag) in deployments {
		let manifest_digest = query!(
			r#"
			SELECT
				manifest_digest
			FROM
				docker_registry_repository_tag
			WHERE
				repository_id = $1 AND
				tag = $2;
			"#,
			repository_id.clone(),
			image_tag
		)
		.fetch_optional(&mut *connection)
		.await?
		.map(|pgrow| pgrow.get::<String, _>("manifest_digest"));

		let manifest_digest = match manifest_digest {
			Some(value) => value,
			None => continue,
		};

		query!(
			r#"
			INSERT INTO
				deployment_deploy_history(
					deployment_id,
					image_digest,
					repository_id,
					created
				)
			VALUES
				($1, $2, $3, $4);
			"#,
			&deployment_id,
			&manifest_digest,
			repository_id,
			&Utc::now()
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			UPDATE
				deployment
			SET
				current_live_digest = $1
			WHERE
				id = $2;
			"#,
			&manifest_digest,
			&deployment_id
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}

async fn create_deployment_config_file(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE deployment_config_mounts(
			path TEXT NOT NULL
				CONSTRAINT deployment_config_mounts_chk_path_valid
					CHECK(path ~ '^[a-zA-Z0-9_\-\.\(\)]+$'),
			file BYTEA NOT NULL,
			deployment_id UUID NOT NULL
				CONSTRAINT deployment_config_mounts_fk_deployment_id
					REFERENCES deployment(id),
			CONSTRAINT deployment_config_mounts_pk PRIMARY KEY(
				deployment_id,
				path
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}

async fn update_dns_record_name_constraint_regexp(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE patr_domain_dns_record
		DROP CONSTRAINT patr_domain_dns_record_chk_name_is_valid;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE patr_domain_dns_record
		ADD CONSTRAINT patr_domain_dns_record_chk_name_is_valid CHECK(
			name ~ '^((\*)|((\*\.)?(([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])\.)*([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])))$' OR
			name = '@'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn add_is_configured_for_managed_urls(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE managed_url
		ADD COLUMN is_configured BOOLEAN NOT NULL
		DEFAULT FALSE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Remove the default value for the is_configured column
	query!(
		r#"
		ALTER TABLE managed_url
		ALTER COLUMN is_configured DROP DEFAULT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			managed_url
		SET
			is_configured = TRUE
		WHERE
			domain_id IN (
				SELECT
					id
				FROM
					workspace_domain
				WHERE
					nameserver_type = 'internal' AND
					is_verified = TRUE
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	let external_managed_urls = query!(
		r#"
		SELECT
			managed_url.id,
			managed_url.workspace_id
		FROM
			managed_url
		INNER JOIN
			workspace_domain
		ON
			managed_url.domain_id = workspace_domain.id
		WHERE
			workspace_domain.nameserver_type = 'external';
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.get::<Uuid, _>("id"), row.get::<Uuid, _>("workspace_id")))
	.collect::<Vec<_>>();

	if external_managed_urls.is_empty() {
		return Ok(());
	}

	// Kubernetes config
	let kubernetes_config = Config::from_custom_kubeconfig(
		Kubeconfig {
			preferences: None,
			clusters: vec![NamedCluster {
				name: config.kubernetes.cluster_name.clone(),
				cluster: Some(Cluster {
					server: Some(config.kubernetes.cluster_url.clone()),
					insecure_skip_tls_verify: None,
					certificate_authority: None,
					certificate_authority_data: Some(
						config.kubernetes.certificate_authority_data.clone(),
					),
					proxy_url: None,
					extensions: None,
					..Default::default()
				}),
			}],
			auth_infos: vec![NamedAuthInfo {
				name: config.kubernetes.auth_name.clone(),
				auth_info: Some(AuthInfo {
					username: Some(config.kubernetes.auth_username.clone()),
					token: Some(config.kubernetes.auth_token.clone().into()),
					..Default::default()
				}),
			}],
			contexts: vec![NamedContext {
				name: config.kubernetes.context_name.clone(),
				context: Some(Context {
					cluster: config.kubernetes.cluster_name.clone(),
					user: config.kubernetes.auth_username.clone(),
					extensions: None,
					namespace: None,
				}),
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

	let certificate_resource = ApiResource {
		group: "cert-manager.io".to_string(),
		version: "v1".to_string(),
		api_version: "cert-manager.io/v1".to_string(),
		kind: "certificate".to_string(),
		plural: "certificates".to_string(),
	};

	for (managed_url_id, workspace_id) in external_managed_urls {
		let cert_exists = match Api::<DynamicObject>::namespaced_with(
			kubernetes_client.clone(),
			workspace_id.as_str(),
			&certificate_resource,
		)
		.get(&format!("certificate-{}", managed_url_id))
		.await
		{
			Err(kube::Error::Api(kube::error::ErrorResponse {
				code: 404,
				..
			})) => Ok(false),
			Err(err) => Err(err),
			Ok(_) => Ok(true),
		}?;

		if cert_exists {
			Api::<DynamicObject>::namespaced_with(
				kubernetes_client.clone(),
				workspace_id.as_str(),
				&certificate_resource,
			)
			.delete(
				&format!("certificate-{}", managed_url_id),
				&DeleteParams::default(),
			)
			.await?;
		}
	}

	Ok(())
}

async fn fix_july_billing_issues(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	// delete the bills calculated for july month
	query!(
		r#"
		DELETE FROM
			transaction
		WHERE
			transaction_type = 'bill' AND
			month = 7;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
