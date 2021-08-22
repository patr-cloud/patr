use std::{array::IntoIter, env};

use clap::crate_version;
use reqwest::multipart::{Form, Part};
use semver::Version;
use serde_json::{json, Value};
use tokio::fs;

#[tokio::main]
async fn main() {
	let crate_version = crate_version!();
	println!("Creating release for version {}...", crate_version);

	let branch = env::var("DRONE_BRANCH").expect("DRONE_BRANCH is not set");

	let system_host = env::var("DRONE_SYSTEM_HOSTNAME")
		.expect("DRONE_SYSTEM_HOSTNAME is not set");
	let system_proto =
		env::var("DRONE_SYSTEM_PROTO").expect("DRONE_SYSTEM_PROTO is not set");

	let repo_owner =
		env::var("DRONE_REPO_OWNER").expect("DRONE_REPO_OWNER is not set");
	let repo_name =
		env::var("DRONE_REPO_NAME").expect("DRONE_REPO_NAME is not set");

	let gitea_ip = env::var("GITEA_IP").expect("GITEA_IP is not set");
	let gitea_token = env::var("GITEA_TOKEN").expect("GITEA_TOKEN not set");
	let release_version = {
		let version = Version::parse(crate_version)
			.expect("unable to parse crate version");
		format!(
			"v{}.{}.{}{}",
			version.major,
			version.minor,
			version.patch,
			if branch == "staging" { "-beta" } else { "" }
		)
	};

	let client = reqwest::Client::builder()
		.resolve(
			&system_host,
			format!("{}:443", gitea_ip)
				.parse()
				.expect("cannot convert IP address to SocketAddr"),
		)
		.build()
		.expect("unable to build reqwest client");
	let url = format!(
		"{}://{}/api/v1/repos/{}/{}/releases",
		system_proto, system_host, repo_owner, repo_name
	);
	println!("Posting to {}", url);
	let response = client
		.post(url)
		.header("Authorization", format!("token {}", gitea_token))
		.json(&json!({
			"name": release_version,
			"prerelease": branch == "staging",
			"tag_name": release_version,
			"target_commitish": branch
		}))
		.send()
		.await
		.expect("unable to send request")
		.text()
		.await
		.expect("unable to parse response as text");
	println!("Release created. Got response: {}", response);
	let response: Value = serde_json::from_str(&response)
		.expect("unable to parse response as JSON");

	println!("Uploading assets...");
	let release_id = response
		.get("id")
		.expect("cannot find ID in response")
		.as_u64()
		.expect("ID in response is not an integer");
	for (name, asset) in IntoIter::new([
		("assets.zip", "./assets.zip"),
		("api", "./target/release/api"),
	]) {
		println!("Uploading {}...", name);
		let response = client
			.post(format!(
				"{}://{}/api/v1/repos/{}/{}/releases/{}/assets",
				system_proto, system_host, repo_owner, repo_name, release_id
			))
			.header("Authorization", format!("token {}", gitea_token))
			.multipart(
				Form::new().text("name", name).part(
					"attachment",
					Part::bytes(
						fs::read(asset).await.expect(&format!(
							"unable to read file `{}`",
							asset
						)),
					),
				),
			)
			.send()
			.await
			.expect("unable to send request");
		if response.status().is_success() {
			println!("Successfully uploaded {}", name);
		} else {
			panic!("Error uploading asset: {:#?}", response);
		}
	}
}
