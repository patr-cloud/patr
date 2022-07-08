use std::fmt::Display;

use api_models::utils::Uuid;
use k8s_openapi::{
	api::core::v1::{
		PersistentVolumeClaim,
		PersistentVolumeClaimSpec,
		ResourceRequirements,
	},
	apimachinery::pkg::api::resource::Quantity,
};
use kube::{api::ObjectMeta, Api};

use crate::{
	models::{CiFlow, Kind, Step},
	rabbitmq::{BuildId, BuildStep, BuildStepId},
	service,
	utils::{settings::Settings, Error},
};

pub mod github;

pub struct Netrc {
	pub machine: String,
	pub login: String,
	pub password: String,
}

impl Display for Netrc {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"machine {} login {} password {}",
			self.machine, self.login, self.password
		)
	}
}

pub async fn create_ci_pipeline(
	ci_flow: CiFlow,
	repo_clone_url: &str,
	repo_name: &str,
	branch_name: &str,
	netrc: Option<Netrc>,
	build_id: BuildId,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	let build_name = build_id.get_pvc_name();
	log::debug!("request_id: {request_id} - Creating a ci pipeline for build `{build_name}`");

	let kube_client = service::get_kubernetes_config(config).await?;
	Api::<PersistentVolumeClaim>::namespaced(
		kube_client,
		build_id.workspace_id.as_str(),
	)
	.create(
		&Default::default(),
		&PersistentVolumeClaim {
			metadata: ObjectMeta {
				name: Some(build_name.clone()),
				..Default::default()
			},
			spec: Some(PersistentVolumeClaimSpec {
				access_modes: Some(vec!["ReadWriteOnce".to_string()]),
				storage_class_name: Some("do-block-storage".to_string()),
				resources: Some(ResourceRequirements {
					requests: Some(
						[(
							"storage".to_string(),
							Quantity("1Gi".to_string()), // TODO
						)]
						.into(),
					),
					..Default::default()
				}),
				volume_mode: Some("Filesystem".to_string()),
				..Default::default()
			}),
			..Default::default()
		},
	)
	.await?;

	// queue clone job
	service::queue_create_ci_build_step(
		BuildStep {
			id: BuildStepId { build_id: build_id.clone(), step_id: 0 },
			image: "alpine/git".to_string(),
			env_vars: vec![],
			commands: vec!	[
				format!(r#"echo "{}" > ~/.netrc"#, netrc.map_or("".to_string(), |netrc| netrc.to_string())),
				r#"cd "/mnt/workdir/""#.to_string(),
				"set -x".to_string(),
				format!(
					r#"git clone --filter=tree:0 --single-branch --branch="{branch_name}" "{repo_clone_url}""#
				),
			],
		},
		config,
		request_id
	)
	.await?;

	// queue build steps
	let Kind::Pipeline(pipeline) = ci_flow.kind;
	for (
		step_id, // TODO
		Step {
			name: _,
			image,
			commands,
			env,
		},
	) in pipeline.steps.into_iter().enumerate()
	{
		let step_id = 1 + step_id as i32;

		service::queue_create_ci_build_step(
			BuildStep {
				id: BuildStepId {
					build_id: build_id.clone(),
					step_id,
				},
				image,
				env_vars: env,
				commands: [
					format!(r#"cd "/mnt/workdir/{repo_name}""#),
					"set -x".to_owned(),
				]
				.into_iter()
				.chain(commands.into_iter())
				.collect::<Vec<_>>(),
			},
			config,
			request_id,
		)
		.await?;
	}

	// queue clean up jobs
	service::queue_clean_ci_build_pipeline(build_id, config, request_id)
		.await?;

	log::debug!("request_id: {request_id} - Successfully created a ci pipeline for build `{build_name}`");
	Ok(())
}
