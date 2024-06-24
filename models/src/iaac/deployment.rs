use std::{collections::BTreeMap, convert::Infallible};

use either::Either;
use serde::{Deserialize, Serialize};

use crate::{
	api::workspace::deployment::{
		DeploymentProbe,
		EnvironmentVariableValue,
		ExposedPortType,
		PatrRegistry,
	},
	prelude::*,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IaacDeployment {
	pub name: String,
	pub image: IaacDeploymentImage,
	pub region: String,
	#[serde(
		alias = "spec",
		alias = "specs",
		alias = "resources",
		alias = "limits",
		default
	)]
	pub machine_type: IaacDeploymentMachineType,
	#[serde(default = "default_deploy_on_push")]
	pub deploy_on_push: bool,
	#[serde(alias = "min-scale", alias = "minscale", default = "default_min_scale")]
	pub min_horizontal_scale: u8,
	#[serde(alias = "max-scale", alias = "maxscale", default = "default_max_scale")]
	pub max_horizontal_scale: u8,
	#[serde(alias = "port")]
	pub ports: IaacDeploymentPorts,
	#[serde(default, alias = "env", alias = "envVars")]
	pub environment_variables: IaacDeploymentEnvVars,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub startup_probe: Option<DeploymentProbe>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub liveness_probe: Option<DeploymentProbe>,
	#[serde(alias = "configs", default, skip_serializing_if = "BTreeMap::is_empty")]
	pub config_mounts: BTreeMap<String, String>,
}

fn default_deploy_on_push() -> bool {
	true
}

fn default_min_scale() -> u8 {
	1
}

fn default_max_scale() -> u8 {
	2
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(try_from = "&str", rename_all = "snake_case", untagged)]
pub enum IaacDeploymentImage {
	PatrRegistry {
		#[serde(alias = "server")]
		registry: PatrRegistry,
		#[serde(alias = "repo", with = "either::serde_untagged")]
		repository: Either<Uuid, String>,
		tag: String,
	},
	ExternalRegistry {
		#[serde(alias = "server")]
		registry: String,
		#[serde(alias = "repo")]
		repository: String,
		tag: String,
	},
}

impl TryFrom<&str> for IaacDeploymentImage {
	type Error = Infallible;

	fn try_from(value: &str) -> Result<Self, Self::Error> {
		let (first, second) = if let Some(split) = value.split_once('/') {
			split
		} else {
			("docker.io", value)
		};

		let valid_repo = first
			.chars()
			.all(|c| c.is_ascii_lowercase() || c.is_numeric() || c == '-' || c == '_');

		let (registry, repository) = if valid_repo {
			("docker.io", value)
		} else {
			(first, second)
		};

		let (repository, tag) = if let Some(split) = repository.split_once(':') {
			split
		} else {
			(repository, "latest")
		};

		Ok(match registry {
			"registry.patr.cloud" => IaacDeploymentImage::PatrRegistry {
				registry: PatrRegistry,
				repository: if let Ok(uuid) = Uuid::parse_str(repository) {
					Either::Left(uuid)
				} else {
					Either::Right(repository.to_string())
				},
				tag: tag.to_string(),
			},
			registry => IaacDeploymentImage::ExternalRegistry {
				registry: registry.to_string(),
				repository: repository.to_string(),
				tag: tag.to_string(),
			},
		})
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(try_from = "&str", rename_all = "snake_case", deny_unknown_fields)]
pub struct IaacDeploymentMachineType {
	pub cpu: IaacDeploymentCpu,
	pub ram: IaacDeploymentRam,
}

impl TryFrom<&str> for IaacDeploymentMachineType {
	type Error = &'static str;

	fn try_from(value: &str) -> Result<Self, Self::Error> {
		let Some((cpu, ram)) = value.split_once(' ') else {
			return Err("machine type must be of the format: `1vCPU 1GB RAM`");
		};

		Ok(Self {
			cpu: cpu.to_string().try_into()?,
			ram: ram.to_string().try_into()?,
		})
	}
}

impl Default for IaacDeploymentMachineType {
	fn default() -> Self {
		Self {
			cpu: IaacDeploymentCpu("1vCPU".to_string()),
			ram: IaacDeploymentRam(1024 * 1024 * 1024),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(try_from = "String", into = "String", deny_unknown_fields)]
pub struct IaacDeploymentCpu(String);

impl TryFrom<String> for IaacDeploymentCpu {
	type Error = &'static str;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		if let Ok(num) = value.parse::<u8>() {
			return Ok(Self(format!("{}vCPU", num)));
		}

		if let Ok(num) = value.parse::<f32>() {
			return Ok(Self(format!("{:.1}vCPU", num)));
		}

		let value = value.to_lowercase();

		if let Some(Ok(num)) = value.strip_suffix("vcpu").map(|num| num.parse::<u8>()) {
			return Ok(Self(format!("{}vCPU", num)));
		}

		if let Some(Ok(num)) = value.strip_suffix("vcpu").map(|num| num.parse::<f32>()) {
			return Ok(Self(format!("{:.1}vCPU", num)));
		}

		if let Some(Ok(num)) = value.strip_suffix("cpu").map(|num| num.parse::<u8>()) {
			return Ok(Self(format!("{}vCPU", num)));
		}

		if let Some(Ok(num)) = value.strip_suffix("cpu").map(|num| num.parse::<f32>()) {
			return Ok(Self(format!("{:.1}vCPU", num)));
		}

		Err("invalid cpu requirement. Must be of the format `1vCPU`")
	}
}

impl Into<String> for IaacDeploymentCpu {
	fn into(self) -> String {
		self.0
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(try_from = "String", into = "String", deny_unknown_fields)]
pub struct IaacDeploymentRam(u64);

impl TryFrom<String> for IaacDeploymentRam {
	type Error = &'static str;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		if let Ok(num) = value.parse::<u64>() {
			return Ok(Self(num));
		}

		let value = value.to_lowercase();

		let value = if let Some(value) = value.strip_suffix(" ram") {
			value
		} else {
			value.as_str()
		};

		let value = if let Some(value) = value.strip_suffix("b") {
			value
		} else {
			value
		};

		if let Some(Ok(num)) = value.strip_suffix("g").map(|num| num.parse::<u16>()) {
			return Ok(Self((num as u64) * 1000 * 1000 * 1000));
		}

		if let Some(Ok(num)) = value.strip_suffix("gi").map(|num| num.parse::<u16>()) {
			return Ok(Self((num as u64) * 1024 * 1024 * 1024));
		}

		if let Some(Ok(num)) = value.strip_suffix("m").map(|num| num.parse::<u32>()) {
			return Ok(Self((num as u64) * 1000 * 1000));
		}

		if let Some(Ok(num)) = value.strip_suffix("mi").map(|num| num.parse::<u32>()) {
			return Ok(Self((num as u64) * 1024 * 1024));
		}

		if let Some(Ok(num)) = value.strip_suffix("k").map(|num| num.parse::<u32>()) {
			return Ok(Self((num as u64) * 1000));
		}

		if let Some(Ok(num)) = value.strip_suffix("ki").map(|num| num.parse::<u32>()) {
			return Ok(Self((num as u64) * 1024));
		}

		if let Some(Ok(num)) = value.strip_suffix("bytes").map(|num| num.parse::<u64>()) {
			return Ok(Self(num));
		}

		if let Some(Ok(num)) = value.strip_suffix("b").map(|num| num.parse::<u64>()) {
			return Ok(Self(num));
		}

		Err("invalid ram requirement. Must be of the format `1GB/GiB/MB/MiB/KB/KiB/B/Bytes`")
	}
}

impl Into<String> for IaacDeploymentRam {
	fn into(self) -> String {
		let Self(bytes) = self;

		// GB
		if bytes % 1000_000_000 == 0 {
			return format!("{}GB RAM", bytes / 1000_000_000);
		}

		// GiB
		if bytes >= (1024 * 1024 * 1024) {
			return if bytes % (1024 * 1024 * 1024) == 0 {
				format!("{}GiB RAM", bytes / (1024 * 1024 * 1024))
			} else {
				format!(
					"{:.1}GiB RAM",
					(bytes as f64) / (1024f64 * 1024f64 * 1024f64)
				)
			};
		}

		// MB
		if bytes % 1000_000 == 0 {
			return format!("{}MB RAM", bytes / 1000_000);
		}

		// MiB
		if bytes >= (1024 * 1024) {
			return if bytes % (1024 * 1024) == 0 {
				format!("{}MiB RAM", bytes / (1024 * 1024))
			} else {
				format!("{:.1}MiB RAM", (bytes as f64) / (1024f64 * 1024f64))
			};
		}

		// KB
		if bytes % 1000 == 0 {
			return format!("{}KB RAM", bytes / 1000);
		}

		// KiB
		if bytes >= (1024) {
			return if bytes % 1024 == 0 {
				format!("{}KiB RAM", bytes / 1024)
			} else {
				format!("{:.1}KiB RAM", (bytes as f64) / 1024f64)
			};
		}

		format!("{}B RAM", bytes)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(try_from = "OneOrMore<String>")]
pub struct IaacDeploymentPorts(BTreeMap<StringifiedU16, ExposedPortType>);

impl TryFrom<OneOrMore<String>> for IaacDeploymentPorts {
	type Error = &'static str;

	fn try_from(value: OneOrMore<String>) -> Result<Self, Self::Error> {
		fn parse_one_port(port: String) -> Result<(u16, ExposedPortType), &'static str> {
			if let Ok(num) = port.trim().parse::<u16>() {
				return Ok((num, ExposedPortType::Http));
			}

			if let Some((port, r#type)) = port.split_once(':') {
				return Ok((
					port.trim()
						.parse::<u16>()
						.map_err(|_| "port must be of the format 8080: http")?,
					r#type
						.trim()
						.to_lowercase()
						.parse::<ExposedPortType>()
						.map_err(|_| "port must be of the format 8080: http")?,
				));
			}

			if let Some((port, r#type)) = port.split_once('=') {
				return Ok((
					port.parse::<u16>()
						.map_err(|_| "port must be of the format 8080=http")?,
					r#type
						.to_lowercase()
						.parse::<ExposedPortType>()
						.map_err(|_| "port must be of the format 8080=http")?,
				));
			}

			Err("port must be of the format 8080: http or 8080=HTTP")
		}

		match value {
			OneOrMore::One(string) => {
				vec![string]
			}
			OneOrMore::Multiple(many) => many,
		}
		.into_iter()
		.map(parse_one_port)
		.map(|port| port.map(|(port, r#type)| (StringifiedU16::from(port), r#type)))
		.collect::<Result<_, _>>()
		.map(Self)
	}
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(try_from = "Vec<String>")]
pub struct IaacDeploymentEnvVars(BTreeMap<String, EnvironmentVariableValue>);

impl TryFrom<Vec<String>> for IaacDeploymentEnvVars {
	type Error = &'static str;

	fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
		fn parse_one_env(env: String) -> Result<(String, EnvironmentVariableValue), &'static str> {
			if let Some((key, value)) = env.split_once('=') {
				return Ok((
					key.trim().to_string(),
					EnvironmentVariableValue::String(value.trim().to_string()),
				));
			}

			Err("environment variable must be of the format KEY=VALUE")
		}

		value
			.into_iter()
			.map(parse_one_env)
			.collect::<Result<_, _>>()
			.map(Self)
	}
}

#[cfg(test)]
mod tests {
	use either::Either;

	use crate::{
		iaac::IaacDeploymentImage,
		models::workspace::deployment::PatrRegistry,
		utils::Uuid,
	};

	#[test]
	fn assert_iaac_deployment_image_parsing_works() {
		for (string, value) in [
			(
				"registry.patr.cloud/workspace-id/api:stable",
				IaacDeploymentImage::PatrRegistry {
					registry: PatrRegistry,
					repository: Either::Right("workspace-id/api".to_string()),
					tag: "stable".to_string(),
				},
			),
			(
				"registry.patr.cloud/workspace-id/api",
				IaacDeploymentImage::PatrRegistry {
					registry: PatrRegistry,
					repository: Either::Right("workspace-id/api".to_string()),
					tag: "latest".to_string(),
				},
			),
			(
				"registry.patr.cloud/api:stable",
				IaacDeploymentImage::PatrRegistry {
					registry: PatrRegistry,
					repository: Either::Right("api".to_string()),
					tag: "stable".to_string(),
				},
			),
			(
				"registry.patr.cloud/api",
				IaacDeploymentImage::PatrRegistry {
					registry: PatrRegistry,
					repository: Either::Right("api".to_string()),
					tag: "latest".to_string(),
				},
			),
			(
				"registry.patr.cloud/01234567890123456789abcdefabcdef:stable",
				IaacDeploymentImage::PatrRegistry {
					registry: PatrRegistry,
					repository: Either::Left(
						Uuid::parse_str("01234567890123456789abcdefabcdef").unwrap(),
					),
					tag: "stable".to_string(),
				},
			),
			(
				"registry.patr.cloud/01234567890123456789abcdefabcdef",
				IaacDeploymentImage::PatrRegistry {
					registry: PatrRegistry,
					repository: Either::Left(
						Uuid::parse_str("01234567890123456789abcdefabcdef").unwrap(),
					),
					tag: "latest".to_string(),
				},
			),
			(
				"workspace-id/api:stable",
				IaacDeploymentImage::ExternalRegistry {
					registry: "docker.io".to_string(),
					repository: "workspace-id/api".to_string(),
					tag: "stable".to_string(),
				},
			),
			(
				"workspace-id/api",
				IaacDeploymentImage::ExternalRegistry {
					registry: "docker.io".to_string(),
					repository: "workspace-id/api".to_string(),
					tag: "latest".to_string(),
				},
			),
			(
				"api:stable",
				IaacDeploymentImage::ExternalRegistry {
					registry: "docker.io".to_string(),
					repository: "api".to_string(),
					tag: "stable".to_string(),
				},
			),
			(
				"api",
				IaacDeploymentImage::ExternalRegistry {
					registry: "docker.io".to_string(),
					repository: "api".to_string(),
					tag: "latest".to_string(),
				},
			),
		] {
			let parsed: IaacDeploymentImage = string.try_into().unwrap();
			assert_eq!(parsed, value);
		}
	}
}
