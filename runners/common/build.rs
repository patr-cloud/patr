//! Build script for the `runners/common` crate.
//!
//! For the following binaries, check if it exists in `../assets/binaries`. If
//! it does not, download it from the specified URL. Do this for:
//! - nginx-linux-amd64
//! - nginx-linux-arm64
//! - nginx-windows-amd64.exe
//! - nginx-darwin-amd64
//! - nginx-darwin-arm64
//! - cloudflare-linux-amd64
//! - cloudflare-linux-arm64
//! - cloudflare-windows-amd64.exe
//! - cloudflare-darwin-amd64
//! - cloudflare-darwin-arm64

#![expect(dead_code, unused_variables)]

use std::{
	env,
	fs::{self, File},
	os::unix::fs::OpenOptionsExt,
	process::Command,
};

use build_print::custom_println;
use flate2::read::GzDecoder;
use reqwest::redirect::Policy;
use tar::Archive;

fn main() {
	let client = reqwest::blocking::Client::builder()
		.redirect(Policy::limited(10))
		.build()
		.expect("Failed to create HTTP client");

	// download_cloudflare_binaries(&client);
	// download_nginx_binaries(&client);

	println!("cargo:rerun-if-changed=build.rs");
	println!(
		"cargo:rerun-if-changed={}/../../assets/binaries/*",
		env!("CARGO_MANIFEST_DIR")
	);
}

/// Download the cloudflared binaries if they do not exist.
fn download_cloudflare_binaries(client: &reqwest::blocking::Client) {
	const CLOUDFLARED_URL: &str =
		"https://github.com/cloudflare/cloudflared/releases/latest/2025.5.0/";

	for (binary, url) in [
		(
			"cloudflared-linux-amd64",
			format!("{}cloudflared-linux-amd64", CLOUDFLARED_URL),
		),
		(
			"cloudflared-linux-arm64",
			format!("{}cloudflared-linux-arm64", CLOUDFLARED_URL),
		),
		(
			"cloudflared-windows-amd64.exe",
			format!("{}cloudflared-windows-amd64.exe", CLOUDFLARED_URL),
		),
		(
			"cloudflared-darwin-amd64.tgz",
			format!("{}cloudflared-darwin-amd64.tgz", CLOUDFLARED_URL),
		),
		(
			"cloudflared-darwin-arm64.tgz",
			format!("{}cloudflared-darwin-arm64.tgz", CLOUDFLARED_URL),
		),
	] {
		if fs::exists(format!(
			"{}/../../assets/binaries/{binary}",
			env!("CARGO_MANIFEST_DIR")
		))
		.expect("Failed to check file existence")
		{
			custom_println!("Removing", green, "binary `{binary}` exists. Removing it.");

			fs::remove_file(format!(
				"{}/../../assets/binaries/{binary}",
				env!("CARGO_MANIFEST_DIR")
			))
			.unwrap_or_else(|err| panic!("Failed to remove file: {binary}: {err}"));
		}
		custom_println!("Fetching", green, "binary `{binary}`");

		let mut response = client
			.get(&url)
			.send()
			.unwrap_or_else(|err| panic!("Failed to download file: {url}: {err}"))
			.error_for_status()
			.unwrap_or_else(|err| panic!("Failed to download file: {url}: {err}"));

		if binary.ends_with(".tgz") {
			Archive::new(GzDecoder::new(response))
				.unpack(format!(
					"{}/../../assets/binaries/",
					env!("CARGO_MANIFEST_DIR")
				))
				.unwrap_or_else(|err| panic!("Failed to unpack file: {binary}: {err}"));
			fs::rename(
				format!(
					"{}/../../assets/binaries/cloudflared",
					env!("CARGO_MANIFEST_DIR")
				),
				format!(
					"{}/../../assets/binaries/{}",
					env!("CARGO_MANIFEST_DIR"),
					binary.trim_end_matches(".tgz")
				),
			)
			.unwrap_or_else(|err| {
				panic!(
					"Failed to rename file: {}: {err}",
					binary.trim_end_matches(".tgz")
				)
			});
		} else {
			let mut file = File::options()
				.append(true)
				.create(true)
				.custom_flags(0o755)
				.open(format!(
					"{}/../../assets/binaries/{binary}",
					env!("CARGO_MANIFEST_DIR")
				))
				.unwrap_or_else(|err| panic!("Failed to open file: {binary}: {err}"));

			custom_println!(
				"Downloaded",
				green,
				"wrote {} bytes.",
				response
					.copy_to(&mut file)
					.unwrap_or_else(|err| panic!("Failed to write file: {binary}: {err}"))
			);
		}
	}
}

/// Download the nginx binaries if they do not exist.
fn download_nginx_binaries(client: &reqwest::blocking::Client) {
	client
		.get("https://nginx.org/download/nginx-1.28.0.tar.gz")
		.send()
		.expect("Failed to download file: nginx")
		.error_for_status()
		.expect("Failed to download file: nginx")
		.copy_to(
			&mut File::create(format!("{}/nginx.tar.gz", env::var("OUT_DIR").unwrap()))
				.expect("Failed to download file: nginx"),
		)
		.expect("Failed to write file: nginx.tar.gz");

	let mut archive = Archive::new(
		File::open(format!("{}/nginx.tar.gz", env::var("OUT_DIR").unwrap()))
			.expect("Failed to open file: nginx.tar.gz"),
	);
	archive
		.unpack(format!("{}/nginx-unpacked", env::var("OUT_DIR").unwrap()))
		.expect("Failed to unpack file: nginx.tar.gz");
	Command::new("sh")
		.arg(concat!(
			"./configure",
			" --with-cc=musl-gcc",
			" --with-ld-opt='-static'",
			" --with-pcre=/usr/include/",
			" --with-zlib=/usr/include/",
		))
		.env("CC", "musl-gcc")
		.current_dir(format!(
			"{}/nginx-unpacked/nginx-1.28.0",
			env::var("OUT_DIR").unwrap()
		))
		.output()
		.expect("Failed to run configure script");
}
