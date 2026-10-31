use config::{Config, File, FileFormat};

use crate::prelude::*;

/// A runner config as `patr runner setup` wrote it before the self-hosted mode
/// was removed: it still carries the `mode` tag and the `bindAddress`, along
/// with the Docker runner's own keys.
const LEGACY_CONFIG: &str = r#"{
	"mode": "managed",
	"workspaceId": "6a1e0c9f5b0d4f0e8d2a4b7c9e1f3a5b",
	"runnerId": "0f9e8d7c6b5a49382716051f2e3d4c5b",
	"apiToken": "test-token",
	"database": {
		"file": "/home/ubuntu/.local/share/patr/cli/runner.docker.db",
		"connectionLimit": 10
	},
	"bindAddress": "127.0.0.1:4000",
	"dockerSwarmListenAddr": "127.0.0.1:2377",
	"ingressHttpListenPort": 80,
	"ingressHttpsListenPort": 443,
	"runnerExposureType": {
		"type": "private"
	},
	"enableIpv6": true
}"#;

/// A legacy config still parses, both through `serde_json` (what `patr runner
/// run` uses) and through the `config` crate (what [`RunnerSettings::parse`]
/// uses), with the removed keys ignored.
#[test]
fn legacy_config_parses() {
	let from_json = serde_json::from_str::<RunnerSettings<()>>(LEGACY_CONFIG)
		.expect("legacy config should parse with serde_json");
	let from_config = Config::builder()
		.add_source(File::from_str(LEGACY_CONFIG, FileFormat::Json))
		.build()
		.expect("legacy config should load with the config crate")
		.try_deserialize::<RunnerSettings<()>>()
		.expect("legacy config should parse with the config crate");

	for settings in [from_json, from_config] {
		assert_eq!(
			settings.workspace_id,
			Uuid::parse_str("6a1e0c9f5b0d4f0e8d2a4b7c9e1f3a5b").unwrap()
		);
		assert_eq!(
			settings.runner_id,
			Uuid::parse_str("0f9e8d7c6b5a49382716051f2e3d4c5b").unwrap()
		);
		assert_eq!(settings.api_token.0.token(), "test-token");
		assert_eq!(
			settings.database.file,
			"/home/ubuntu/.local/share/patr/cli/runner.docker.db"
		);
		assert_eq!(settings.database.connection_limit, 10);
	}
}
