use std::env;

use clap::crate_version;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::{
	fs::{self, OpenOptions},
	io::AsyncWriteExt,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RepositoryResponse {
	id: u64,
	name: String,
	full_name: String,
	release_counter: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ReleaseResponse {
	id: u64,
	tag_name: String,
	target_commitish: String,
	name: String,
	body: String,
	draft: bool,
	prerelease: bool,
	assets: Vec<ReleaseAssetResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ReleaseAssetResponse {
	id: u64,
	name: String,
	browser_download_url: String,
}

#[tokio::main]
async fn main() {
	let crate_version = crate_version!();
	println!(
		"Testing all migrations up till version {}...",
		crate_version
	);

	let _branch = env::var("DRONE_BRANCH").expect("DRONE_BRANCH is not set");
	let system_host = "develop.vicara.co".to_string();
	let system_proto = "https".to_string();
	let repo_owner =
		env::var("DRONE_REPO_OWNER").expect("DRONE_REPO_OWNER is not set");
	let repo_name =
		env::var("DRONE_REPO_NAME").expect("DRONE_REPO_NAME is not set");
	let gitea_token = env::var("GITEA_TOKEN").expect("GITEA_TOKEN not set");

	let client = reqwest::Client::new();

	println!();
	println!("Getting information of versions");
	let releases = get_total_number_of_releases(
		&system_proto,
		&system_host,
		&repo_owner,
		&repo_name,
		&gitea_token,
		&client,
	)
	.await;

	println!("Found {} releases in the repository. Testing migrations against all of them", releases);
	println!();

	for i in (1..releases).step_by(10) {
		println!("Getting details of release {} to {}", i, i + 10);
		let releases = get_release_details_of_releases(
			i,
			10,
			&system_proto,
			&system_host,
			&repo_owner,
			&repo_name,
			&gitea_token,
			&client,
		)
		.await;

		for release in releases {
			handle_release(release, &gitea_token, &client).await;
		}
	}
}

async fn get_total_number_of_releases(
	system_proto: &str,
	system_host: &str,
	repo_owner: &str,
	repo_name: &str,
	gitea_token: &str,
	client: &Client,
) -> u64 {
	client
		.get(format!(
			"{}://{}/api/v1/repos/{}/{}",
			system_proto, system_host, repo_owner, repo_name
		))
		.header("Authorization", format!("token {}", gitea_token))
		.send()
		.await
		.expect("unable to send request")
		.json::<RepositoryResponse>()
		.await
		.expect("unable to parse response as json")
		.release_counter
}

async fn get_release_details_of_releases(
	page: u64,
	limit: u64,
	system_proto: &str,
	system_host: &str,
	repo_owner: &str,
	repo_name: &str,
	gitea_token: &str,
	client: &Client,
) -> Vec<ReleaseResponse> {
	client
		.get(format!(
			"{}://{}/api/v1/repos/{}/{}/releases",
			system_proto, system_host, repo_owner, repo_name
		))
		.header("Authorization", format!("token {}", gitea_token))
		.query(&[("page", page), ("limit", limit)])
		.send()
		.await
		.expect("unable to send request")
		.json::<Vec<ReleaseResponse>>()
		.await
		.expect("unable to parse response as json")
}

async fn handle_release(
	release: ReleaseResponse,
	gitea_token: &str,
	client: &Client,
) {
	println!("Checking database details of version {}", release.tag_name);

	let database_dump = release
		.assets
		.into_iter()
		.find(|asset| asset.name == "database.sql");
	let database_dump = if let Some(dump) = database_dump {
		dump
	} else {
		println!(
			"{} doesn't contain a database.sql file. Moving on...",
			release.tag_name
		);
		return;
	};

	println!(
		"{} contains a database.sql file. Downloading the database dump...",
		release.tag_name
	);

	// Download and perform migration
	if fs::metadata("./database.sql").await.is_ok() {
		fs::remove_file("./database.sql")
			.await
			.expect("unable to delete database.sql");
	}
	let mut local_dump_file = OpenOptions::new()
		.write(true)
		.create(true)
		.open("./database.sql")
		.await
		.expect("unable to open database.sql file for writing");

	let mut dump_file_stream = client
		.get(&database_dump.browser_download_url)
		.header("Authorization", format!("token {}", gitea_token))
		.send()
		.await
		.expect("unable to send request")
		.bytes_stream();
	while let Some(Ok(bytes)) = dump_file_stream.next().await {
		local_dump_file
			.write(bytes.as_ref())
			.await
			.expect("unable to write data to database.sql");
	}

	println!("Database dump downloaded");

	println!();
	println!("Testing migrations against version {}...", release.tag_name);

	// TODO
	// Clear postgres
	// Create a fresh database creation
	// Dump the fresh database creation to fresh.sql
	// Clear postgres again
	// Restore database.sql to postgres
	// Run the application to run migrations
	// Dump the new database to migrated.sql
	// Diff check migrated.sql and fresh.sql
	// Use the following command to diff check
	// diff -s -y -W 70 --suppress-common-lines migrated.sql fresh.sql
}
