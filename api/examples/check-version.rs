use std::env;

use api_macros::version;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ReleaseVersion {
	tag_name: String,
	name: String
}

async fn does_version_exist(version: &str) -> bool {
	println!("Validating package version...");

	let system_host = "develop.vicara.co";
	let system_proto = "https";

	let repo_owner =
		env::var("DRONE_REPO_OWNER").expect("DRONE_REPO_OWNER is not set");
	let repo_name =
		env::var("DRONE_REPO_NAME").expect("DRONE_REPO_NAME is not set");

	let gitea_token = env::var("GITEA_TOKEN").expect("GITEA_TOKEN not set");

	let mut version_exists = false;
	let mut is_response_empty = false;
	let mut page: u8 = 1;

	let client = reqwest::Client::new();

	while !version_exists {
		let url = format!(
			"{}://{}/api/v1/repos/{}/{}/releases?limit=20&page={}",
			system_proto, system_host, repo_owner, repo_name, page
		);

		let response = client
			.get(url)
			.header("Authorization", format!("token {}", gitea_token))
			.send()
			.await
			.expect("unable to send request")
			.json::<Vec<ReleaseVersion>>()
			.await
			.expect("unable to parse response as json");
		if response.len() == 0 {
			is_response_empty = true;
			break;
		}
		for release in response.iter() {
			if version == release.tag_name || version == release.name {
				version_exists = true;
			}
		}
		page += 1;
	}
	version_exists || !is_response_empty
}

#[tokio::main]
async fn main() {
	let crate_version = version!();

	let branch = env::var("DRONE_BRANCH").expect("DRONE_BRANCH is not set");

	let release_version = {
		format!(
			"v{}.{}.{}{}",
			crate_version.major,
			crate_version.minor,
			crate_version.patch,
			if branch == "staging" { "-beta" } else { "" }
		)
	};

	let version_exists = does_version_exist(&release_version).await;

	if version_exists {
		panic!(
			"Version not updated. {} is already released!",
			release_version
		);
	} else {
		println!("Version validated successfully...")
	}
}
