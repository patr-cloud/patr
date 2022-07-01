use std::fmt::Display;

use k8s_openapi::api::core::v1::Pod;
use kube::{api::PostParams, Api};
use serde_json::json;

use crate::{
	models::{CiFlow, Kind, Step},
	utils::Error,
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
	ci_file: impl AsRef<[u8]>,
	repo_clone_url: &str,
	repo_name: &str,
	branch_name: &str,
	netrc: Option<Netrc>,
	unique_pipeline_name: &str,
	kube_client: kube::Client,
) -> Result<(), Error> {
	log::debug!("Create a pod to run custom ci commands");

	let ci_flow: CiFlow = serde_yaml::from_slice(ci_file.as_ref())?;

	let ci_steps = std::iter::once({
        // first clone the repo
        let clone_repo_command = [
			&format!(r#"echo "{}" > ~/.netrc"#, netrc.map_or("".to_string(), |netrc| netrc.to_string())),
            r#"cd "/mnt/workdir/""#,
            "set -x",
            &format!(
                r#"git clone --filter=tree:0 --single-branch --branch="{branch_name}" "{repo_clone_url}""#
            ),
        ]
        .join("\n");

        json!({
          "name": "git-clone",
          "image": "alpine/git",
          "volumeMounts": [
            {
              "name": "vol-workdir",
              "mountPath": "/mnt/workdir"
            }
          ],
          "command": [
              "sh",
              "-ce",
              clone_repo_command
          ]
        })
    })
    .chain({
        // now add the ci steps defined by user
        let Kind::Pipeline(pipeline) = ci_flow.kind;
        pipeline.steps.into_iter().map(
            |Step {
                 name, // TODO: slugify names and make sure it will be allowed in k8s (unique)
                 image,
                 commands,
                 env
             }| {
                let commands_str = [
                    format!(r#"cd "/mnt/workdir/{repo_name}""#),
                    "set -x".to_owned(),
                ]
                .into_iter()
                .chain(commands.into_iter())
                .collect::<Vec<_>>()
                .join("\n"); // TODO: use iter_intersperse once it got stabilized

                if let Some(env) = env {
					json!({
						"name": name,
						"image": image,
						"volumeMounts": [
                            {
                                "name": "vol-workdir",
                                "mountPath": "/mnt/workdir"
                            }
						],
						"env": env,
						"command": [
							"sh",
							"-ce",
							commands_str
						]
					})
                } else {
					json!({
						"name": name,
						"image": image,
						"volumeMounts": [
                            {
                                "name": "vol-workdir",
                                "mountPath": "/mnt/workdir"
                            }
						],
						"command": [
							"sh",
							"-ce",
							commands_str
						]
					})
				}
            },
        )
    })
    .collect::<Vec<_>>();

	let pod_spec: Pod = serde_json::from_value(json!({
	  "apiVersion": "v1",
	  "kind": "Pod",
	  "metadata": {
		"name": unique_pipeline_name
	  },
	  "spec": {
		"restartPolicy": "Never",
		"volumes": [
		  {
			"name": "vol-workdir",
			"emptyDir": {}
		  }
		],
		"initContainers": ci_steps,
		"containers": [
		  {
			"name": "echo-ci-success",
			"image": "alpine/git",
			"volumeMounts": [
			  {
				"name": "vol-workdir",
				"mountPath": "/mnt/workdir"
			  }
			],
			"command": [
				"sh",
				"-ce",
				r#"echo "CI steps completed successfully""#
			]
		  }
		]
	  }
	}))?;

	let pods_api = Api::<Pod>::namespaced(kube_client, "kavin");
	pods_api.create(&PostParams::default(), &pod_spec).await?;

	log::debug!("successfully created a ci pipeline in k8s");

	// TODO: clean up pod after running the ci steps

	Ok(())
}
