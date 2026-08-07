use std::{
	collections::BTreeMap,
	convert::Infallible,
	hash::{Hash, Hasher},
	str::FromStr,
};

use serde::{Deserialize, Serialize};

use crate::{api::workspace::deployment::*, iaac::MaybeExternallySourced, prelude::*};

/// The IaaC definition for a deployment. This is basically the same as the
/// [`Deployment`] struct, but with a few fields (like status, current live
/// digest) not present, as they are not needed for the Iaac file, as well as
/// other utilities for parsing strings easier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IaacDeployment {
	/// The ID of the deployment that needs to be patched (used to identify the
	/// deployment if the name has changed)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub id: Option<Uuid>,
	/// The name of the deployment
	pub name: MaybeExternallySourced<String>,
	/// The image to use for the deployment. This can be a Patr registry image
	/// or an external registry image.
	pub image: MaybeExternallySourced<IaacDeploymentImage>,
	/// Which runner to deploy the deployment to.
	pub runner: MaybeExternallySourced<String>,
	/// Whether the deployment should be deployed on push to the repository
	pub deploy_on_push: MaybeExternallySourced<bool>,
	/// The minimum number of instances to run for the deployment.
	#[serde(alias = "min-scale", alias = "minscale")]
	pub min_horizontal_scale: MaybeExternallySourced<u16>,
	/// The maximum number of instances to run for the deployment.
	#[serde(alias = "max-scale", alias = "maxscale")]
	pub max_horizontal_scale: MaybeExternallySourced<u16>,
	/// The ports that the deployment exposes. This is a map of port numbers to
	/// the type of port (HTTP, HTTPS, TCP, etc.).
	#[serde(
		alias = "port",
		default,
		skip_serializing_if = "IaacDeploymentPorts::is_empty"
	)]
	pub ports: IaacDeploymentPorts,
	/// The environment variables that the deployment has. This is a map of
	/// environment variable names to their values.
	#[serde(
		default,
		alias = "env",
		alias = "envs",
		alias = "envVars",
		skip_serializing_if = "IaacDeploymentEnvVars::is_empty"
	)]
	pub environment_variables: IaacDeploymentEnvVars,
	/// The startup probe for the deployment. This is used to check if the
	/// deployment is ready to serve traffic.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub startup_probe: Option<DeploymentProbe>,
	/// The liveness probe for the deployment. This is used to check if the
	/// deployment is still alive and should be restarted if it is not.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub liveness_probe: Option<DeploymentProbe>,
	/// The config mounts for the deployment. This is a map of config names to
	/// the paths where the configs should be mounted in the deployment.
	#[serde(
		alias = "configs",
		alias = "config",
		default,
		skip_serializing_if = "BTreeMap::is_empty"
	)]
	pub config_mounts: BTreeMap<String, String>,
}

impl Hash for IaacDeployment {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id
			.map(|id| id.hash(state))
			.unwrap_or_else(|| self.name.hash(state));
	}
}

/// The Iaac deployment image that is used in the Iaac file. This can either be
/// a Patr registry image or an external registry image. The Patr registry image
/// is used for images that are hosted on the Patr registry, while the external
/// registry image is used for images that are hosted on an external registry,
/// such as Docker Hub.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
	try_from = "String",
	into = "String",
	rename_all = "snake_case",
	untagged
)]
pub enum IaacDeploymentImage {
	/// A Patr registry image, which is an image that is hosted on the Patr
	/// registry.
	PatrRegistry {
		/// The Patr registry that the image is hosted on. This is always
		/// `registry.patr.cloud`.
		registry: PatrRegistry,
		/// The repository of the image. This can either be a UUID or a name.
		repository: String,
		/// The tag of the image. This is always `latest` if not specified.
		tag: String,
	},
	/// An external registry image, which is an image that is hosted on an
	/// external registry, such as Docker Hub.
	ExternalRegistry {
		/// The registry that the image is hosted on. This can be any valid
		/// Docker registry, such as Docker Hub or a private registry.
		registry: String,
		/// The repository of the image
		repository: String,
		/// The tag of the image. This is always `latest` if not specified.
		tag: String,
	},
}

/// The default tag for the IaacDeploymentImage. This is set to `latest` by
/// default, meaning that the image will be pulled with the `latest` tag if no
/// tag is specified.
fn default_image_tag() -> &'static str {
	"latest"
}

impl From<String> for IaacDeploymentImage {
	fn from(value: String) -> Self {
		let Ok(parsed) = value.parse();
		parsed
	}
}

impl FromStr for IaacDeploymentImage {
	type Err = Infallible;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
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

		let (repository, tag) = if let Some((repo, tag)) = repository.split_once(':') {
			(repo, tag)
		} else {
			(repository, default_image_tag())
		};

		Ok(match registry {
			"registry.patr.cloud" => IaacDeploymentImage::PatrRegistry {
				registry: PatrRegistry,
				repository: repository.to_string(),
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

impl From<IaacDeploymentImage> for String {
	fn from(value: IaacDeploymentImage) -> String {
		match value {
			IaacDeploymentImage::PatrRegistry {
				registry,
				repository,
				tag,
			} => {
				format!("{}/{}/{}", registry, repository, tag)
			}
			IaacDeploymentImage::ExternalRegistry {
				registry,
				repository,
				tag,
			} => {
				format!("{}/{}/{}", registry, repository, tag)
			}
		}
	}
}

/// A helper type to parse the ports of a deployment. This is a map of port
/// numbers to the type of port (HTTP, HTTPS, TCP, etc.). The port numbers
/// are stored as `StringifiedU16`, which is a wrapper around `u16`
/// that implements `Serialize` and `Deserialize` to ensure that the port
/// numbers are always serialized as strings.
///
/// The allowed formats for the ports are:
/// - `8080: http`
/// - `8080=HTTP`
/// - `8080/http`
/// - `8080` (defaults to HTTP)
///
/// The type of the port can be one of:
/// - `http`
/// - `tcp`
/// - `udp`
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IaacDeploymentPorts(BTreeMap<StringifiedU16, ExposedPortType>);

impl IaacDeploymentPorts {
	/// Get the inner map of the IaacDeploymentPorts.
	pub fn into_inner(self) -> BTreeMap<StringifiedU16, ExposedPortType> {
		self.0
	}

	/// Check if the IaacDeploymentPorts is empty. Returns `true` if the inner
	/// map is empty, `false` otherwise.
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
}

impl Serialize for IaacDeploymentPorts {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.0.serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for IaacDeploymentPorts {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		BTreeMap::<StringifiedU16, ExposedPortType>::deserialize(deserializer).map(Self)
	}
}

impl FromStr for IaacDeploymentPorts {
	type Err = &'static str;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		IaacDeploymentPorts::try_from(OneOrMore::One(value.to_string()))
	}
}

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
					port.trim()
						.parse::<u16>()
						.map_err(|_| "port must be of the format 8080=http")?,
					r#type
						.trim()
						.to_lowercase()
						.parse::<ExposedPortType>()
						.map_err(|_| "port must be of the format 8080=http")?,
				));
			}

			if let Some((port, r#type)) = port.split_once('/') {
				return Ok((
					port.trim()
						.parse::<u16>()
						.map_err(|_| "port must be of the format 8080/http")?,
					r#type
						.trim()
						.to_lowercase()
						.parse::<ExposedPortType>()
						.map_err(|_| "port must be of the format 8080/http")?,
				));
			}

			Err("port must be of the format 8080: http, 8080=HTTP or 8080/http")
		}

		value
			.into_iter()
			.map(parse_one_port)
			.map(|port| port.map(|(port, r#type)| (StringifiedU16::from(port), r#type)))
			.collect::<Result<_, _>>()
			.map(Self)
	}
}

/// A helper type to parse the environment variables of a deployment. This is a
/// map of environment variable names to their values. The environment variables
/// can be defined in the Iaac file in the following formats:
/// - `KEY=VALUE`
///
/// An environment variable value can be either a raw string or a reference to a
/// secret.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IaacDeploymentEnvVars(
	BTreeMap<String, MaybeExternallySourced<EnvironmentVariableValue>>,
);

impl IaacDeploymentEnvVars {
	/// Get the inner map of the IaacDeploymentEnvVars.
	pub fn into_inner(self) -> BTreeMap<String, MaybeExternallySourced<EnvironmentVariableValue>> {
		self.0
	}

	/// Check if the IaacDeploymentEnvVars is empty. Returns `true` if the
	/// inner map is empty, `false` otherwise.
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
}

impl Serialize for IaacDeploymentEnvVars {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.0.serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for IaacDeploymentEnvVars {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		BTreeMap::<String, _>::deserialize(deserializer).map(Self)
	}
}

impl TryFrom<Vec<String>> for IaacDeploymentEnvVars {
	type Error = &'static str;

	fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
		fn parse_one_env(
			env: String,
		) -> Result<(String, MaybeExternallySourced<EnvironmentVariableValue>), &'static str> {
			if let Some((key, value)) = env.split_once('=') {
				return Ok((
					key.trim().to_string(),
					MaybeExternallySourced::Value(EnvironmentVariableValue::String(
						value.trim().to_string(),
					)),
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
	use serde_test::{Token, assert_tokens};

	use crate::{api::workspace::deployment::PatrRegistry, iaac::IaacDeploymentImage};

	#[test]
	fn assert_iaac_deployment_image_parsing_works() {
		for (string, value) in [
			(
				"registry.patr.cloud/workspace-id/api:stable",
				IaacDeploymentImage::PatrRegistry {
					registry: PatrRegistry,
					repository: "workspace-id/api".to_string(),
					tag: "stable".to_string(),
				},
			),
			(
				"registry.patr.cloud/workspace-id/api",
				IaacDeploymentImage::PatrRegistry {
					registry: PatrRegistry,
					repository: "workspace-id/api".to_string(),
					tag: "latest".to_string(),
				},
			),
			(
				"registry.patr.cloud/api:stable",
				IaacDeploymentImage::PatrRegistry {
					registry: PatrRegistry,
					repository: "api".to_string(),
					tag: "stable".to_string(),
				},
			),
			(
				"registry.patr.cloud/api",
				IaacDeploymentImage::PatrRegistry {
					registry: PatrRegistry,
					repository: "api".to_string(),
					tag: "latest".to_string(),
				},
			),
			(
				"registry.patr.cloud/01234567890123456789abcdefabcdef:stable",
				IaacDeploymentImage::PatrRegistry {
					registry: PatrRegistry,
					repository: "01234567890123456789abcdefabcdef".to_string(),
					tag: "stable".to_string(),
				},
			),
			(
				"registry.patr.cloud/01234567890123456789abcdefabcdef",
				IaacDeploymentImage::PatrRegistry {
					registry: PatrRegistry,
					repository: "01234567890123456789abcdefabcdef".to_string(),
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
			let parsed: IaacDeploymentImage = string.parse().unwrap();
			assert_eq!(parsed, value);
		}
	}

	#[test]
	fn assert_serialization_of_iaac_deployment_image() {
		assert_tokens(
			&IaacDeploymentImage::PatrRegistry {
				registry: PatrRegistry,
				repository: "workspace-id/api".to_string(),
				tag: "stable".to_string(),
			},
			&[
				Token::Struct {
					name: "IaacDeploymentImage",
					len: 3,
				},
				Token::Str("registry"),
				Token::Str("registry.patr.cloud"),
				Token::Str("repository"),
				Token::Str("workspace-id/api"),
				Token::Str("tag"),
				Token::Str("stable"),
				Token::StructEnd,
			],
		);

		assert_tokens(
			&IaacDeploymentImage::ExternalRegistry {
				registry: "docker.io".to_string(),
				repository: "grafana/grafana-oss".to_string(),
				tag: "latest".to_string(),
			},
			&[
				Token::Struct {
					name: "IaacDeploymentImage",
					len: 3,
				},
				Token::Str("registry"),
				Token::Str("docker.io"),
				Token::Str("repository"),
				Token::Str("grafana/grafana-oss"),
				Token::Str("tag"),
				Token::Str("latest"),
				Token::StructEnd,
			],
		);
	}
}
