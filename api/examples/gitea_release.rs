use std::env;

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

	let client = reqwest::Client::new();
	let response = client
		.post(format!(
			"https://{}/repos/{}/{}/releases",
			env::var("DRONE_SYSTEM_HOST").expect("DRONE_SYSTEM_HOST not set"),
			env::var("DRONE_REPO_OWNER").expect("DRONE_REPO_OWNER not set"),
			env::var("DRONE_REPO_NAME").expect("DRONE_REPO_NAME not set")
		))
		.header(
			"Authorization",
			format!(
				"token {}",
				env::var("GITEA_TOKEN").expect("GITEA_TOKEN not set")
			),
		)
		.json(&json!({
			"name": release_version,
			"prerelease": branch == "staging",
			"tag_name": release_version,
			"target_commitish": branch
		}))
		.send()
		.await
		.expect("unable to send request")
		.json::<Value>()
		.await
		.expect("unable to parse response as JSON");
	println!("Release created. Got response: {:#?}", response);

	println!("Uploading assets...");
	let release_id = response
		.get("id")
		.expect("cannot find ID in response")
		.as_str()
		.expect("ID in response is not a string");
	for (name, asset) in [
		("assets.zip", "./assets.zip"),
		("api", "./target/release/api"),
	] {
		println!("Uploading {}...", name);
		let response = client
			.post(format!(
				"https://{}/repos/{}/{}/releases/{}/assets",
				env::var("DRONE_SYSTEM_HOST")
					.expect("DRONE_SYSTEM_HOST not set"),
				env::var("DRONE_REPO_OWNER").expect("DRONE_REPO_OWNER not set"),
				env::var("DRONE_REPO_NAME").expect("DRONE_REPO_NAME not set"),
				release_id
			))
			.header(
				"Authorization",
				format!(
					"token {}",
					env::var("GITEA_TOKEN").expect("GITEA_TOKEN not set")
				),
			)
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
