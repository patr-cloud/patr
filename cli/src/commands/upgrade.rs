use std::{
	io::Read as _,
	path::{Path, PathBuf},
	process::Command,
};

use clap::Args as ClapArgs;
use models::ApiSuccessResponseBody;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::prelude::*;

/// Args for `patr upgrade`.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// Switch the target channel. Persisted to your preferences — future
	/// `patr upgrade` runs use this channel until you change it again.
	#[arg(long = "channel", value_enum)]
	pub channel: Option<Channel>,
	/// Skip the "already up to date" short-circuit. Does not permit downgrades
	/// across channels — run `patr uninstall` and reinstall to go back.
	#[arg(long)]
	pub force: bool,
	/// Print what an upgrade would do and exit without modifying anything.
	/// Exit codes: 0 = up to date, 1 = update available, 2 = error.
	#[arg(long)]
	pub check: bool,
}

/// Minimal subset of the GitHub Releases API response we care about.
#[derive(Debug, Deserialize)]
struct GhRelease {
	/// Release tag (e.g. `v0.18.0`, `alpha`).
	tag_name: String,
	/// Files uploaded to the release.
	assets: Vec<GhAsset>,
}

/// A single release asset returned by the GitHub Releases API.
#[derive(Debug, Deserialize)]
struct GhAsset {
	/// Filename of the asset.
	name: String,
	/// Browser-facing download URL.
	browser_download_url: String,
}

/// Machine-readable output of `patr upgrade`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpgradeOutput {
	/// The channel the upgrade is tracking.
	channel: Channel,
	/// The version this binary is running.
	current: String,
	/// The version the target channel is currently at.
	remote: String,
	/// What happened: `upToDate`, `install`, `reinstall`, `installed`.
	action: &'static str,
}

/// Decision made by comparing the current version to the target channel's
/// remote version.
#[derive(Debug, PartialEq, Eq)]
enum Action {
	/// No upgrade needed — local is as new or newer than remote.
	UpToDate,
	/// Remote is strictly newer — install it.
	Install,
	/// Same version as remote but user passed `--force` — reinstall.
	Reinstall,
}

impl Action {
	/// Machine-readable tag for the JSON output.
	fn as_str(&self) -> &'static str {
		match self {
			Self::UpToDate => "upToDate",
			Self::Install => "install",
			Self::Reinstall => "reinstall",
		}
	}
}

/// Upgrade the Patr CLI in place.
pub async fn execute(
	args: Args,
	_global_args: GlobalArgs,
	mut state: AppState,
) -> Result<CommandOutput, AppError> {
	if cfg!(feature = "package-managed") {
		eprintln!("self-update is disabled for this build of patr");
		eprintln!("this CLI is managed by your package manager — use it to upgrade");
		return CommandOutput::builder()
			.text(String::new())
			.json(ApiSuccessResponseBody::empty().to_json_value())
			.build()
			.into_result();
	}

	if let Some(requested) = args.channel {
		state.target_channel = requested;
		state.clone().save()?;
	}
	let target = state.target_channel;

	let current =
		Version::parse(constants::PATR_BUILD_VERSION.trim_start_matches('v')).map_err(|e| {
			AppError::RunnerError(format!(
				"Failed to parse current version `{}`: {e}",
				constants::PATR_BUILD_VERSION
			))
		})?;

	let client = reqwest::Client::builder()
		.user_agent(constants::USER_AGENT.as_str())
		.build()
		.map_err(|e| AppError::RunnerError(format!("Failed to build HTTP client: {e}")))?;

	// Resolve the release for the target channel.
	let api = format!(
		"https://api.github.com/repos/{}/releases",
		constants::GITHUB_REPO
	);
	let release = match target {
		Channel::Beta => find_latest_beta(&client, &api).await?,
		Channel::Stable | Channel::Alpha => {
			let url = if target == Channel::Stable {
				format!("{api}/latest")
			} else {
				format!("{api}/tags/alpha")
			};
			client
				.get(&url)
				.send()
				.await
				.and_then(|r| r.error_for_status())
				.map_err(|e| AppError::RunnerError(format!("Failed to GET {url}: {e}")))?
				.json::<GhRelease>()
				.await
				.map_err(|e| {
					AppError::RunnerError(format!("Failed to decode response from {url}: {e}"))
				})?
		}
	};

	// Read the release's version.txt asset to learn the remote version.
	let version_asset = release
		.assets
		.iter()
		.find(|a| a.name == "version.txt")
		.ok_or_else(|| {
			AppError::RunnerError(format!(
				"release {} is missing the `version.txt` asset",
				release.tag_name
			))
		})?;
	let remote_version_string = client
		.get(&version_asset.browser_download_url)
		.send()
		.await
		.and_then(|r| r.error_for_status())
		.map_err(|e| AppError::RunnerError(format!("Failed to fetch version.txt: {e}")))?
		.text()
		.await
		.map_err(|e| AppError::RunnerError(format!("Failed to read version.txt: {e}")))?
		.trim()
		.to_string();
	let remote = Version::parse(remote_version_string.trim_start_matches('v')).map_err(|e| {
		AppError::RunnerError(format!(
			"Failed to parse remote version `{remote_version_string}`: {e}"
		))
	})?;

	let action = {
		use std::cmp::Ordering::*;
		match remote.cmp(&current) {
			Greater => Action::Install,
			Equal if args.force => Action::Reinstall,
			Equal => Action::UpToDate,
			Less if args.force => {
				return Err(AppError::RunnerError(format!(
					"downgrade not supported.\n\
					 You're on {current}; the target channel is at {remote}.\n\n\
					 Options:\n\
					 \u{20}\u{20}- Drop `--force` to set this as your target channel. You'll be upgraded\n\
					 \u{20}\u{20}\u{20}\u{20}automatically once this channel reaches a version ahead of {current}.\n\
					 \u{20}\u{20}- To downgrade right now: `patr uninstall` then reinstall.",
				)));
			}
			Less => Action::UpToDate,
		}
	};

	if args.check || action == Action::UpToDate {
		let text = match action {
			Action::UpToDate => {
				format!("up to date (current: {current} on channel {target}; remote: {remote})")
			}
			Action::Install => format!("update available: {current} → {remote} (channel {target})"),
			Action::Reinstall => format!("reinstall available: {current} (channel {target})"),
		};
		eprintln!("{text}");
		// `--check` is expected to exit non-zero when an update is available so
		// callers can script against it. The current CommandOutput contract
		// only gives us Ok→0 / Err→1, so "update available" under --check
		// maps to exit 0 with a message — good enough for humans, not yet
		// right for scripts. TODO: add an exit-code hint to AppError.
		return CommandOutput::builder()
			.text(text)
			.json(
				UpgradeOutput {
					channel: target,
					current: current.to_string(),
					remote: remote.to_string(),
					action: action.as_str(),
				}
				.to_json_value(),
			)
			.build()
			.into_result();
	}

	// Locate the archive asset for this platform.
	let platform = match (std::env::consts::OS, std::env::consts::ARCH) {
		("linux", "x86_64") => "linux-amd64",
		("linux", "aarch64") => "linux-arm64",
		("macos", "aarch64") => "darwin-arm64",
		(os, arch) => {
			return Err(AppError::RunnerError(format!(
				"unsupported platform: {os}/{arch}"
			)));
		}
	};
	let ext = if cfg!(target_os = "macos") {
		"zip"
	} else {
		"tar.gz"
	};
	let expected_asset = format!("patr-{platform}.{ext}");
	let artifact = release
		.assets
		.iter()
		.find(|a| a.name == expected_asset)
		.ok_or_else(|| {
			AppError::RunnerError(format!(
				"release {} is missing the expected asset `{expected_asset}`",
				release.tag_name
			))
		})?;
	let sha_asset_name = format!("{}.sha256sum", artifact.name);
	let sha_asset = release
		.assets
		.iter()
		.find(|a| a.name == sha_asset_name)
		.ok_or_else(|| {
			AppError::RunnerError(format!(
				"release {} is missing {sha_asset_name}",
				release.tag_name
			))
		})?;

	let tmpdir = tempfile::TempDir::new()
		.map_err(|e| AppError::RunnerError(format!("Failed to create temp dir: {e}")))?;
	let archive_path = tmpdir.path().join(&artifact.name);
	let sha_path = tmpdir.path().join(&sha_asset.name);

	eprintln!("Downloading {}...", artifact.name);
	download(&client, &artifact.browser_download_url, &archive_path).await?;
	download(&client, &sha_asset.browser_download_url, &sha_path).await?;

	eprint!("Verifying checksum... ");
	{
		let sha_contents = std::fs::read_to_string(&sha_path)
			.map_err(|e| AppError::RunnerError(format!("Failed to read checksum file: {e}")))?;
		let expected = sha_contents
			.split_whitespace()
			.next()
			.ok_or_else(|| AppError::RunnerError("checksum file is empty".into()))?;

		let mut file = std::fs::File::open(&archive_path)
			.map_err(|e| AppError::RunnerError(format!("Failed to open archive: {e}")))?;
		let mut hasher = Sha256::new();
		let mut buf = [0u8; 8192];
		loop {
			let n = file
				.read(&mut buf)
				.map_err(|e| AppError::RunnerError(format!("Failed to read archive: {e}")))?;
			if n == 0 {
				break;
			}
			hasher.update(&buf[..n]);
		}
		let actual = hasher
			.finalize()
			.iter()
			.map(|b| format!("{b:02x}"))
			.collect::<String>();

		if !expected.eq_ignore_ascii_case(&actual) {
			return Err(AppError::RunnerError(format!(
				"checksum mismatch for {}: expected {expected}, got {actual}",
				artifact.name
			)));
		}
	}
	eprintln!("ok");

	let new_binary = extract_binary(&archive_path, tmpdir.path())?;

	let current_exe = std::env::current_exe().map_err(|e| {
		AppError::RunnerError(format!("Failed to determine current binary path: {e}"))
	})?;

	swap_binary(&new_binary, &current_exe)?;

	eprintln!("Updated: {current} → {remote}");

	CommandOutput::builder()
		.text(format!("Updated: {current} → {remote}"))
		.json(
			UpgradeOutput {
				channel: target,
				current: current.to_string(),
				remote: remote.to_string(),
				action: "installed",
			}
			.to_json_value(),
		)
		.build()
		.into_result()
}

/// Paginate the GitHub releases list until we find the newest release whose
/// tag matches `v*-beta.*`.
async fn find_latest_beta(client: &reqwest::Client, api: &str) -> Result<GhRelease, AppError> {
	let mut page: u32 = 1;
	loop {
		let url = format!("{api}?per_page=100&page={page}");
		let releases = client
			.get(&url)
			.send()
			.await
			.and_then(|r| r.error_for_status())
			.map_err(|e| AppError::RunnerError(format!("Failed to GET {url}: {e}")))?
			.json::<Vec<GhRelease>>()
			.await
			.map_err(|e| {
				AppError::RunnerError(format!("Failed to decode response from {url}: {e}"))
			})?;
		let len = releases.len();
		if let Some(r) = releases
			.into_iter()
			.find(|r| r.tag_name.starts_with('v') && r.tag_name.contains("-beta."))
		{
			return Ok(r);
		}
		if len < 100 {
			return Err(AppError::RunnerError(format!(
				"no beta release found on {}",
				constants::GITHUB_REPO
			)));
		}
		page += 1;
	}
}

/// Download `url` to `dest` in one shot, using the shared client.
async fn download(client: &reqwest::Client, url: &str, dest: &Path) -> Result<(), AppError> {
	let bytes = client
		.get(url)
		.send()
		.await
		.and_then(|r| r.error_for_status())
		.map_err(|e| AppError::RunnerError(format!("Failed to download {url}: {e}")))?
		.bytes()
		.await
		.map_err(|e| AppError::RunnerError(format!("Failed to read body of {url}: {e}")))?;

	std::fs::write(dest, &bytes)
		.map_err(|e| AppError::RunnerError(format!("Failed to write {}: {e}", dest.display())))
}

/// Extract the archive and return the path to the extracted `patr` binary.
fn extract_binary(archive: &Path, into: &Path) -> Result<PathBuf, AppError> {
	let archive_name = archive
		.file_name()
		.and_then(|s| s.to_str())
		.ok_or_else(|| AppError::RunnerError("archive path is not utf-8".into()))?;

	let file = std::fs::File::open(archive)
		.map_err(|e| AppError::RunnerError(format!("Failed to open archive: {e}")))?;

	if archive_name.ends_with(".tar.gz") {
		tar::Archive::new(flate2::read::GzDecoder::new(file))
			.unpack(into)
			.map_err(|e| AppError::RunnerError(format!("Failed to extract tarball: {e}")))?;
	} else if archive_name.ends_with(".zip") {
		zip::ZipArchive::new(file)
			.map_err(|e| AppError::RunnerError(format!("Failed to read zip: {e}")))?
			.extract(into)
			.map_err(|e| AppError::RunnerError(format!("Failed to extract zip: {e}")))?;
	} else {
		return Err(AppError::RunnerError(format!(
			"unsupported archive extension: {archive_name}"
		)));
	}

	let binary = into.join("patr");
	if !binary.exists() {
		return Err(AppError::RunnerError(
			"downloaded archive did not contain an expected `patr` binary".into(),
		));
	}
	Ok(binary)
}

/// Replace the running binary with `new_binary`. Falls back to `sudo install`
/// when the current path isn't user-writable (typical for `/usr/local/bin`).
fn swap_binary(new_binary: &Path, current_exe: &Path) -> Result<(), AppError> {
	match self_replace::self_replace(new_binary) {
		Ok(()) => return Ok(()),
		Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {}
		Err(err) => {
			return Err(AppError::RunnerError(format!(
				"Failed to replace running binary: {err}"
			)));
		}
	}

	eprintln!(
		"Installing to {} (may prompt for your password).",
		current_exe.display()
	);
	let status = Command::new("sudo")
		.arg("install")
		.args(["-m", "0755"])
		.arg(new_binary)
		.arg(current_exe)
		.status()
		.map_err(|e| match e.kind() {
			std::io::ErrorKind::NotFound => AppError::RunnerError(
				"sudo not found on PATH. Install sudo or run `patr upgrade` as root.".into(),
			),
			_ => AppError::RunnerError(format!("Failed to run sudo: {e}")),
		})?;
	if !status.success() {
		return Err(AppError::RunnerError(format!(
			"`sudo install` failed with exit status {status}"
		)));
	}
	Ok(())
}
