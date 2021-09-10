use std::{env, process::Stdio};

use clap::crate_version;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::{
	fs::{self, OpenOptions},
	io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
	process::Command,
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

// What this does:
// ---------------
// A database would be created from the build command
// Dump existing database to existing.sql
// Clear postgres
// Create a fresh database creation
// Dump the fresh database creation to fresh.sql
// For sanity check, check if fresh.sql is the same as existing.sql
// Get the list of versions which has a "database.sql" file
// For each version:
// - Clear postgres again
// - Restore database.sql to postgres
// - Run the application to run migrations
// - Dump the new database to migrated.sql
// - Diff check migrated.sql and fresh.sql
// - Use the following command to diff check
// - diff -s -y -W 70 --suppress-common-lines migrated.sql fresh.sql

#[tokio::main]
async fn main() {
	let crate_version = crate_version!();
	println!(
		"Testing all migrations up till version {}...",
		crate_version
	);

	let _branch = get_env_var("DRONE_BRANCH");
	let system_host = String::from("develop.vicara.co");
	let system_proto = String::from("https");
	let repo_owner = get_env_var("DRONE_REPO_OWNER");
	let repo_name = get_env_var("DRONE_REPO_NAME");
	let gitea_token = get_env_var("GITEA_TOKEN");
	let db_host = get_env_var("APP_DATABASE_HOST");
	let db_port = get_env_var("APP_DATABASE_PORT");
	let db_username = get_env_var("APP_DATABASE_USER");
	let db_password = get_env_var("APP_DATABASE_PASSWORD");
	let db_database = get_env_var("APP_DATABASE_DATABASE");
	let redis_host = get_env_var("APP_REDIS_HOST");

	println!("Dumping existing database to existing.sql...");
	dump_database(
		&db_host,
		&db_port,
		&db_username,
		&db_password,
		&db_database,
		"existing.sql",
	)
	.await;
	println!("Database dump completed");

	println!("Clearing database...");
	clear_database(
		&db_host,
		&db_port,
		&db_username,
		&db_password,
		&db_database,
	)
	.await;
	println!("Database cleared");

	println!();
	println!("Recreating the database from scratch...");
	run_api_with_only_db(
		&db_host,
		&db_port,
		&db_username,
		&db_password,
		&db_database,
		&redis_host,
	)
	.await;
	println!("Database recreated");

	println!("Dumping freshly created database to fresh.sql...");
	dump_database(
		&db_host,
		&db_port,
		&db_username,
		&db_password,
		&db_database,
		"fresh.sql",
	)
	.await;
	println!("Database dump completed");
	println!();

	println!("If everything is going fine, existing.sql should be exactly the same as fresh.sql");
	println!("Performing sanity check...");
	check_if_files_are_equal("existing.sql", "fresh.sql").await;
	println!("Both files are the exact same. We are sane");
	println!("Running migration tests...");

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
			handle_release(
				release,
				&db_host,
				&db_port,
				&db_username,
				&db_password,
				&db_database,
				&redis_host,
				&gitea_token,
				&client,
			)
			.await;
		}
	}
}

fn get_env_var(env_name: &str) -> String {
	env::var(env_name).expect(&format!("{} is not set", env_name))
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
	db_host: &str,
	db_port: &str,
	db_username: &str,
	db_password: &str,
	db_database: &str,
	redis_host: &str,
	gitea_token: &str,
	client: &Client,
) {
	println!();
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

	println!("Clearing database...");
	clear_database(
		&db_host,
		&db_port,
		&db_username,
		&db_password,
		&db_database,
	)
	.await;
	println!("Database cleared");

	println!("Restoring database dump...");
	restore_database_dump(
		&db_host,
		&db_port,
		&db_username,
		&db_password,
		&db_database,
		"database.sql",
	)
	.await;
	println!("Database dump restored");

	println!("Testing migrations against version {}...", release.tag_name);
	println!("Running api to run migrations...");
	run_api_with_only_db(
		&db_host,
		&db_port,
		&db_username,
		&db_password,
		&db_database,
		redis_host,
	)
	.await;
	println!("Migration successfully completed");

	println!("Dumping migrated database to migrated.sql");
	dump_database(
		&db_host,
		&db_port,
		&db_username,
		&db_password,
		&db_database,
		"migrated.sql",
	)
	.await;
	println!("Migrated database dumped to migrated.sql");

	println!("Checking if migrated.sql is the same as fresh.sql...");
	check_if_files_are_equal("migrated.sql", "fresh.sql").await;
	println!("Both files are the exact same.");
	println!("Migrations successful against version {}", release.tag_name);
	println!();
}

async fn dump_database(
	host: &str,
	port: &str,
	username: &str,
	password: &str,
	database: &str,
	file_name: &str,
) {
	if fs::metadata(file_name).await.is_ok() {
		fs::remove_file(file_name)
			.await
			.expect(&format!("unable to delete {}", file_name));
	}

	let pg_dump = Command::new("pg_dump")
		.arg(format!("--host={}", host))
		.arg(format!("--port={}", port))
		.arg(format!("--username={}", username))
		.arg(format!("--dbname={}", database))
		.env("PGPASSWORD", password)
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.spawn()
		.expect("Unable to spawn pg_dump command");
	let output = pg_dump.stdout.expect("Unable to get stdout from pg_dump");
	let mut line_iterator = BufReader::new(output).lines();

	let mut output_file = OpenOptions::new()
		.write(true)
		.create(true)
		.open(file_name)
		.await
		.expect(&format!("unable to open {} file for writing", file_name));

	while let Ok(Some(line)) = line_iterator.next_line().await {
		output_file
			.write_all(line.as_bytes())
			.await
			.expect(&format!("Unable to write to file {}", file_name));
	}
}

async fn clear_database(
	host: &str,
	port: &str,
	username: &str,
	password: &str,
	database: &str,
) {
	let success = Command::new("psql")
		.arg(format!("--host={}", host))
		.arg(format!("--port={}", port))
		.arg(format!("--username={}", username))
		.arg(format!("--command=\"DROP DATABASE {};\"", database))
		.env("PGPASSWORD", password)
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.spawn()
		.expect("Unable to spawn psql command")
		.wait()
		.await
		.expect("Unable to wait for DROP DATABASE command to complete")
		.success();
	if !success {
		panic!("DROP DATABASE command exited with a non-success exit code");
	}

	let success = Command::new("psql")
		.arg(format!("--host={}", host))
		.arg(format!("--port={}", port))
		.arg(format!("--username={}", username))
		.arg(format!("--command=\"CREATE DATABASE {};\"", database))
		.env("PGPASSWORD", password)
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.spawn()
		.expect("Unable to spawn psql command")
		.wait()
		.await
		.expect("Unable to wait for DROP DATABASE command to complete")
		.success();
	if !success {
		panic!("CREATE DATABASE command exited with a non-success exit code");
	}
}

async fn run_api_with_only_db(
	host: &str,
	port: &str,
	username: &str,
	password: &str,
	database: &str,
	redis_host: &str,
) {
	let success = Command::new("./target/release/api")
		.arg("--db-only")
		.env("APP_DATABASE_HOST", host)
		.env("APP_DATABASE_PORT", port)
		.env("APP_DATABASE_USER", username)
		.env("APP_DATABASE_PASSWORD", password)
		.env("APP_DATABASE_DATABASE", database)
		.env("APP_REDIS_HOST", redis_host)
		.stdin(Stdio::inherit())
		.stdout(Stdio::inherit())
		.spawn()
		.expect("Unable to spawn `api --db-only` command")
		.wait()
		.await
		.expect("Unable to wait for `api --db-only` command to complete")
		.success();
	if !success {
		panic!("`api --db-only` command exited with a non-success exit code");
	}
}

async fn check_if_files_are_equal(first_file: &str, second_file: &str) {
	let success = Command::new("diff")
		.arg("-s")
		.arg("-y")
		.arg("-W")
		.arg("70")
		.arg("--suppress-common-lines")
		.arg(first_file)
		.arg(second_file)
		.stdin(Stdio::inherit())
		.stdout(Stdio::inherit())
		.spawn()
		.expect("Unable to spawn diff command")
		.wait()
		.await
		.expect("Unable to wait for diff command to complete")
		.success();
	if !success {
		panic!("diff command exited with a non-success exit code");
	}
}

async fn restore_database_dump(
	host: &str,
	port: &str,
	username: &str,
	password: &str,
	database: &str,
	file_name: &str,
) {
	let mut psql = Command::new("psql")
		.arg(format!("--host={}", host))
		.arg(format!("--port={}", port))
		.arg(format!("--username={}", username))
		.arg(format!("--dbname={}", database))
		.env("PGPASSWORD", password)
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.spawn()
		.expect("Unable to spawn psql command");
	let input = psql.stdin.as_mut().expect("Unable to get stdout from psql");

	let input_file = OpenOptions::new()
		.read(true)
		.open(file_name)
		.await
		.expect(&format!("unable to open {} file for reading", file_name));
	let mut line_iterator = BufReader::new(input_file).lines();

	while let Ok(Some(line)) = line_iterator.next_line().await {
		input
			.write_all(line.as_bytes())
			.await
			.expect("Unable to write to psql");
	}

	input
		.write_all("exit".as_bytes())
		.await
		.expect("Unable to write to psql");

	let success = psql
		.wait()
		.await
		.expect("unable to wait for psql to exit")
		.success();

	if !success {
		panic!("psql command exited with a non-success exit code");
	}
}
