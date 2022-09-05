use std::env;

use api_macros::version;
use reqwest::multipart::{Form, Part};
use serde_json::{json, Value};
use tokio::fs;

#[tokio::main]
async fn main() {
	let crate_version = version!();
	println!("Creating release for version {}...", crate_version);

	let branch = env::var("DRONE_BRANCH").expect("DRONE_BRANCH is not set");

	let system_host = "develop.vicara.co";
	let system_proto = "https";

	let repo_owner =
		env::var("DRONE_REPO_OWNER").expect("DRONE_REPO_OWNER is not set");
	let repo_name =
		env::var("DRONE_REPO_NAME").expect("DRONE_REPO_NAME is not set");

	let gitea_token = env::var("GITEA_TOKEN").expect("GITEA_TOKEN not set");
	let release_version = {
		format!(
			"v{}.{}.{}{}",
			crate_version.major,
			crate_version.minor,
			crate_version.patch,
			if branch == "staging" { "-beta" } else { "" }
		)
	};

	let client = reqwest::Client::new();
	let url = format!(
		"{}://{}/api/v1/repos/{}/{}/releases",
		system_proto, system_host, repo_owner, repo_name
	);
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
	for (name, asset) in [
		("assets.zip", "./assets.zip"),
		("api", "./target/release/api"),
		("config.sample.json", "./config/dev.sample.json"),
		("database.sql", "./fresh.sql"),
	] {
		println!("Uploading {}...", name);
		let response = client
			.post(format!(
				"{}://{}/api/v1/repos/{}/{}/releases/{}/assets",
				system_proto, system_host, repo_owner, repo_name, release_id
			))
			.header("Authorization", format!("token {}", gitea_token))
			.query(&[("name", name)])
			.multipart(
				Form::new().text("name", name).part(
					"attachment",
					Part::bytes(fs::read(asset).await.unwrap_or_else(|_| {
						panic!("unable to read file `{}`", asset)
					}))
					.file_name(name),
				),
			)
			.send()
			.await
			.expect("unable to send request");
		if response.status().is_success() {
			println!("Successfully uploaded {}", name);
		} else {
			panic!("Error uploading asset: {:#?}", response.text().await);
		}
	}
}
