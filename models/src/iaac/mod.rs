use std::{
	fmt::Display,
	hash::{Hash, Hasher},
	str::FromStr,
};

use either::Either;
use serde::{Deserialize, Serialize};

pub use self::{database::*, deployment::*, domain::*, error::*, managed_url::*};
use crate::{prelude::*, rbac::ResourceType};

/// All database related IaaC structs and functions.
mod database;
/// All deployment related IaaC structs and functions.
mod deployment;
/// All domain related IaaC structs and functions.
mod domain;
/// All Iaac related error types.
mod error;
/// All managed URL related IaaC structs and functions.
mod managed_url;

/// Any resource that can be defined in an Iaac file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct IaacResource {
	/// The resource data, which can be a deployment, database, etc.
	#[serde(flatten)]
	pub data: IaacResourceData,
	/// What the resource depends on, e.g. a database or another deployment.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub depends_on: Option<OneOrMore<Dependency>>,
}

impl IaacResource {
	/// Returns the type of the resource, e.g. `deployment`, `database`, etc.
	pub fn resource_type(&self) -> ResourceType {
		self.data.get_resource_type()
	}

	/// Returns the name of the resource, which is used to identify it in Iaac
	/// files.
	pub fn name(&self) -> &MaybeExternallySourced<String> {
		self.data.name()
	}

	/// Returns the dependencies of the resource, which can be a single
	/// dependency or multiple dependencies. If there are no dependencies, it
	/// returns an empty slice.
	pub fn dependencies(&self) -> &[Dependency] {
		match self.depends_on.as_ref() {
			Some(OneOrMore::One(dependency)) => std::slice::from_ref(dependency),
			Some(OneOrMore::Multiple(dependencies)) => dependencies.as_slice(),
			None => &[],
		}
	}
}

impl Hash for IaacResource {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.data.get_resource_type().hash(state);
		self.data.name().hash(state);
	}
}

/// The Iaac resource that is defined in the Iaac file. This is a particular
/// resource that can be deployed, such as a deployment, database, static site,
/// managed URL, domain, Docker repository, or secret.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum IaacResourceData {
	/// A deployment resource, which is a containerized application.
	Deployment(IaacDeployment),
	/// A domain resource, which is a DNS domain.
	Domain(IaacDomain),
	/// A managed URL resource, which routes traffic to deployments or other
	/// URLs.
	ManagedUrl(IaacManagedUrl),
}

impl IaacResourceData {
	/// Returns the type of the resource, e.g. `deployment`, `database`, etc.
	pub fn get_resource_type(&self) -> ResourceType {
		match self {
			Self::Deployment(_) => ResourceType::Deployment,
			Self::Domain(_) => ResourceType::Domain,
			Self::ManagedUrl(_) => ResourceType::ManagedURL,
		}
	}

	/// Returns the name of the resource, which is used to identify it in Iaac
	/// files.
	pub fn name(&self) -> &MaybeExternallySourced<String> {
		match self {
			Self::Deployment(deployment) => &deployment.name,
			Self::Domain(domain) => &domain.name,
			Self::ManagedUrl(managed_url) => &managed_url.sub_domain,
		}
	}
}

/// A dependency of a resource, which can either be a UUID or a name. This is
/// used to define what a resource depends on, e.g. a database or another
/// deployment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
	/// The type of the resource that this dependency is for, e.g. `deployment`,
	/// `database`, etc. This is only required if the identifier is a name AND
	/// multiple resources with that name exist.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub resource: Option<ResourceType>,
	/// The identifier of the resource that this dependency is for. This can be
	/// either a UUID or a name. If it is a name, it must be unique within the
	/// workspace.
	pub identifier: Either<Uuid, String>,
}

/// A helper type to parse a value that can either be a raw value or a reference
/// from an external source (such as an environment variable). This is used
/// to allow for more flexibility in Iaac files, where values can be defined
/// as either raw strings or references to external sources.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum MaybeExternallySourced<T> {
	/// A raw value. This is used when the value is a simple string or number.
	Value(T),
	/// A value that is sourced from an environment variable.
	FromEnvironment {
		/// The name of the environment variable to source the value from.
		#[serde(alias = "env")]
		from_env: String,
	},
	// Maybe tomorrow we can add more types of external sources, such as
	// from another resource's value, etc
}

impl<T> MaybeExternallySourced<T>
where
	T: FromStr,
	<T as FromStr>::Err: Display,
{
	/// Returns the value of the `MaybeExternallySourced` type, either the raw
	/// value or the value sourced from an environment variable.
	pub fn resolve_value(self) -> Result<T, IaacError> {
		match self {
			Self::Value(value) => Ok(value),
			Self::FromEnvironment { from_env } => std::env::var(&from_env)
				.map(|value| T::from_str(&value))
				.map_err(|_| IaacError::EnvironmentVariableNotFound(from_env))?
				.map_err(|err| IaacError::EnvironmentVariableParseError(err.to_string())),
		}
	}
}

impl<T> Default for MaybeExternallySourced<T>
where
	T: Default,
{
	fn default() -> Self {
		Self::Value(T::default())
	}
}

impl<T> From<T> for MaybeExternallySourced<T> {
	fn from(value: T) -> Self {
		Self::Value(value)
	}
}

#[cfg(test)]
mod tests {
	use serde_json;
	use serde_test::{Token, assert_tokens};

	use super::*;

	#[test]
	fn test_maybe_externally_sourced_value_serialization() {
		let value: MaybeExternallySourced<String> =
			MaybeExternallySourced::Value("test".to_string());

		// Test with serde_test
		assert_tokens(&value, &[Token::Str("test")]);

		// Also test JSON round-trip for completeness
		let json = serde_json::to_string(&value).unwrap();
		assert_eq!(json, "\"test\"");

		let deserialized: MaybeExternallySourced<String> = serde_json::from_str(&json).unwrap();
		assert_eq!(value, deserialized);
	}

	#[test]
	fn test_maybe_externally_sourced_env_serialization() {
		let value: MaybeExternallySourced<String> = MaybeExternallySourced::FromEnvironment {
			from_env: "MY_VAR".to_string(),
		};

		// Test with serde_test
		assert_tokens(
			&value,
			&[
				Token::Struct {
					name: "MaybeExternallySourced",
					len: 1,
				},
				Token::Str("from_env"),
				Token::Str("MY_VAR"),
				Token::StructEnd,
			],
		);

		// Also test JSON round-trip
		let json = serde_json::to_string(&value).unwrap();
		assert_eq!(json, "{\"from_env\":\"MY_VAR\"}");

		let deserialized: MaybeExternallySourced<String> = serde_json::from_str(&json).unwrap();
		assert_eq!(value, deserialized);
	}

	#[test]
	fn test_maybe_externally_sourced_env_alias_serialization() {
		let json = "{\"env\":\"MY_VAR\"}";
		let deserialized: MaybeExternallySourced<String> = serde_json::from_str(&json).unwrap();
		let expected = MaybeExternallySourced::FromEnvironment {
			from_env: "MY_VAR".to_string(),
		};
		assert_eq!(deserialized, expected);
	}

	#[test]
	fn test_maybe_externally_sourced_numeric_types() {
		// Test with integer type
		let int_value: MaybeExternallySourced<i32> = MaybeExternallySourced::Value(42);
		assert_tokens(&int_value, &[Token::I32(42)]);

		// Test with float type
		let float_value: MaybeExternallySourced<f64> = MaybeExternallySourced::Value(3.14);
		assert_tokens(&float_value, &[Token::F64(3.14)]);

		// Test with boolean type
		let bool_value: MaybeExternallySourced<bool> = MaybeExternallySourced::Value(true);
		assert_tokens(&bool_value, &[Token::Bool(true)]);
	}

	#[test]
	fn test_maybe_externally_sourced_env_numeric_serialization() {
		// Test environment sourced integer
		let env_int: MaybeExternallySourced<i32> = MaybeExternallySourced::FromEnvironment {
			from_env: "TEST_INT".to_string(),
		};
		assert_tokens(
			&env_int,
			&[
				Token::Struct {
					name: "MaybeExternallySourced",
					len: 1,
				},
				Token::Str("from_env"),
				Token::Str("TEST_INT"),
				Token::StructEnd,
			],
		);

		// Test environment sourced boolean
		let env_bool: MaybeExternallySourced<bool> = MaybeExternallySourced::FromEnvironment {
			from_env: "TEST_BOOL".to_string(),
		};
		assert_tokens(
			&env_bool,
			&[
				Token::Struct {
					name: "MaybeExternallySourced",
					len: 1,
				},
				Token::Str("from_env"),
				Token::Str("TEST_BOOL"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn test_maybe_externally_sourced_resolve_value() {
		// Test direct value
		let value: MaybeExternallySourced<String> =
			MaybeExternallySourced::Value("test".to_string());
		assert_eq!(value.resolve_value().unwrap(), "test");

		// Test environment variable (this will fail unless the env var is set)
		let env_value: MaybeExternallySourced<String> = MaybeExternallySourced::FromEnvironment {
			from_env: "TEST_VAR".to_string(),
		};
		assert_eq!(env_value.resolve_value().unwrap(), "env_value");
	}

	#[test]
	fn test_dependency_uuid_serialization() {
		let uuid = Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").unwrap();
		let dependency = Dependency {
			resource: Some(ResourceType::Deployment),
			identifier: Either::Left(uuid),
		};

		// Test with serde_test
		assert_tokens(
			&dependency,
			&[
				Token::Struct {
					name: "Dependency",
					len: 2,
				},
				Token::Str("resource"),
				Token::Some,
				Token::UnitVariant {
					name: "ResourceType",
					variant: "deployment",
				},
				Token::Str("identifier"),
				Token::NewtypeVariant {
					name: "Either",
					variant: "Left",
				},
				Token::Str("123e4567e89b12d3a456426614174000"),
				Token::StructEnd,
			],
		);

		// Also test JSON round-trip
		let json = serde_json::to_string(&dependency).unwrap();
		let deserialized: Dependency = serde_json::from_str(&json).unwrap();
		assert_eq!(dependency, deserialized);
	}

	#[test]
	fn test_dependency_name_serialization() {
		let dependency = Dependency {
			resource: None,
			identifier: Either::Right("my-resource".to_string()),
		};

		// Test with serde_test
		assert_tokens(
			&dependency,
			&[
				Token::Struct {
					name: "Dependency",
					len: 1,
				},
				Token::Str("identifier"),
				Token::NewtypeVariant {
					name: "Either",
					variant: "Right",
				},
				Token::Str("my-resource"),
				Token::StructEnd,
			],
		);

		// Also test JSON round-trip
		let json = serde_json::to_string(&dependency).unwrap();
		let deserialized: Dependency = serde_json::from_str(&json).unwrap();
		assert_eq!(dependency, deserialized);
	}

	// // Additional comprehensive serialization/deserialization tests

	// #[test]
	// fn test_maybe_externally_sourced_numeric_types() {
	// 	// Test with integer type
	// 	let int_value: MaybeExternallySourced<i32> =
	// MaybeExternallySourced::Value(42); 	assert_tokens(&int_value,
	// &[Token::I32(42)]);

	// 	// Test with float type
	// 	let float_value: MaybeExternallySourced<f64> =
	// MaybeExternallySourced::Value(3.14); 	assert_tokens(&float_value,
	// &[Token::F64(3.14)]);

	// 	// Test with boolean type
	// 	let bool_value: MaybeExternallySourced<bool> =
	// MaybeExternallySourced::Value(true); 	assert_tokens(&bool_value,
	// &[Token::Bool(true)]); }

	// #[test]
	// fn test_maybe_externally_sourced_env_numeric_serialization() {
	// 	// Test environment sourced integer
	// 	let env_int: MaybeExternallySourced<i32> =
	// MaybeExternallySourced::FromEnvironment { 		from_env:
	// "TEST_INT".to_string(), 	};
	// 	assert_tokens(
	// 		&env_int,
	// 		&[
	// 			Token::Struct {
	// 				name: "MaybeExternallySourced",
	// 				len: 1,
	// 			},
	// 			Token::Str("from_env"),
	// 			Token::Str("TEST_INT"),
	// 			Token::StructEnd,
	// 		],
	// 	);

	// 	// Test environment sourced boolean
	// 	let env_bool: MaybeExternallySourced<bool> =
	// MaybeExternallySourced::FromEnvironment { 		from_env:
	// "TEST_BOOL".to_string(), 	};
	// 	assert_tokens(
	// 		&env_bool,
	// 		&[
	// 			Token::Struct {
	// 				name: "MaybeExternallySourced",
	// 				len: 1,
	// 			},
	// 			Token::Str("from_env"),
	// 			Token::Str("TEST_BOOL"),
	// 			Token::StructEnd,
	// 		],
	// 	);
	// }

	// #[test]
	// fn test_maybe_externally_sourced_env_numeric_resolve() {
	// 	// Test resolving numeric types from environment
	// 	std::env::set_var("TEST_INT", "123");
	// 	let env_int: MaybeExternallySourced<i32> =
	// MaybeExternallySourced::FromEnvironment { 		from_env:
	// "TEST_INT".to_string(), 	};
	// 	assert_eq!(env_int.resolve_value().unwrap(), 123);
	// 	std::env::remove_var("TEST_INT");

	// 	std::env::set_var("TEST_BOOL", "true");
	// 	let env_bool: MaybeExternallySourced<bool> =
	// MaybeExternallySourced::FromEnvironment { 		from_env:
	// "TEST_BOOL".to_string(), 	};
	// 	assert_eq!(env_bool.resolve_value().unwrap(), true);
	// 	std::env::remove_var("TEST_BOOL");
	// }

	// #[test]
	// fn test_maybe_externally_sourced_env_resolve_errors() {
	// 	// Test missing environment variable
	// 	let missing_env: MaybeExternallySourced<String> =
	// MaybeExternallySourced::FromEnvironment { 		from_env:
	// "NONEXISTENT_VAR".to_string(), 	};
	// 	assert!(matches!(
	// 		missing_env.resolve_value(),
	// 		Err(IaacError::EnvironmentVariableNotFound(_))
	// 	));

	// 	// Test parse error
	// 	std::env::set_var("INVALID_INT", "not_a_number");
	// 	let invalid_int: MaybeExternallySourced<i32> =
	// MaybeExternallySourced::FromEnvironment { 		from_env:
	// "INVALID_INT".to_string(), 	};
	// 	assert!(matches!(
	// 		invalid_int.resolve_value(),
	// 		Err(IaacError::EnvironmentVariableParseError(_))
	// 	));
	// 	std::env::remove_var("INVALID_INT");
	// }

	// #[test]
	// fn test_dependency_with_resource_type_serialization() {
	// 	// Test dependency with resource type specified
	// 	let dependency = Dependency {
	// 		resource: Some(ResourceType::Deployment),
	// 		identifier: Either::Right("my-deployment".to_string()),
	// 	};

	// 	// Test with serde_test
	// 	assert_tokens(
	// 		&dependency,
	// 		&[
	// 			Token::Struct {
	// 				name: "Dependency",
	// 				len: 2,
	// 			},
	// 			Token::Str("resource"),
	// 			Token::Some,
	// 			Token::UnitVariant {
	// 				name: "ResourceType",
	// 				variant: "deployment",
	// 			},
	// 			Token::Str("identifier"),
	// 			Token::NewtypeVariant {
	// 				name: "Either",
	// 				variant: "Right",
	// 			},
	// 			Token::Str("my-deployment"),
	// 			Token::StructEnd,
	// 		],
	// 	);

	// 	// Also test JSON round-trip
	// 	let json = serde_json::to_string(&dependency).unwrap();
	// 	assert!(json.contains("\"resource\":\"deployment\""));
	// 	assert!(json.contains("\"identifier\":\"my-deployment\""));

	// 	let deserialized: Dependency = serde_json::from_str(&json).unwrap();
	// 	assert_eq!(dependency, deserialized);
	// }

	// #[test]
	// fn test_dependency_without_resource_type_serialization() {
	// 	// Test dependency without resource type
	// 	let uuid = Uuid::new_v4();
	// 	let dependency = Dependency {
	// 		resource: None,
	// 		identifier: Either::Left(uuid),
	// 	};

	// 	// Test with serde_test
	// 	assert_tokens(
	// 		&dependency,
	// 		&[
	// 			Token::Struct {
	// 				name: "Dependency",
	// 				len: 2,
	// 			},
	// 			Token::Str("resource"),
	// 			Token::None,
	// 			Token::Str("identifier"),
	// 			Token::NewtypeVariant {
	// 				name: "Either",
	// 				variant: "Left",
	// 			},
	// 			Token::Str(&uuid.to_string()),
	// 			Token::StructEnd,
	// 		],
	// 	);

	// 	// Also test JSON round-trip
	// 	let json = serde_json::to_string(&dependency).unwrap();
	// 	assert!(!json.contains("\"resource\""));

	// 	let deserialized: Dependency = serde_json::from_str(&json).unwrap();
	// 	assert_eq!(dependency, deserialized);
	// }

	// #[test]
	// fn test_iaac_resource_with_env_name_serialization() {
	// 	let deployment = IaacDeployment {
	// 		name: MaybeExternallySourced::FromEnvironment {
	// 			from_env: "DEPLOYMENT_NAME".to_string(),
	// 		},
	// 	};

	// 	let resource = IaacResource {
	// 		data: IaacResourceData::Deployment(deployment),
	// 		depends_on: None,
	// 	};

	// 	let json = serde_json::to_string(&resource).unwrap();
	// 	assert!(json.contains("\"from_env\":\"DEPLOYMENT_NAME\""));

	// 	let deserialized: IaacResource = serde_json::from_str(&json).unwrap();
	// 	assert_eq!(resource, deserialized);
	// }

	// #[test]
	// fn test_iaac_resource_flattened_serialization() {
	// 	// Test that the deployment fields are flattened into the resource
	// 	let deployment = IaacDeployment {
	// 		name: MaybeExternallySourced::Value("test-app".to_string()),
	// 	};

	// 	let resource = IaacResource {
	// 		data: IaacResourceData::Deployment(deployment),
	// 		depends_on: None,
	// 	};

	// 	let json = serde_json::to_string(&resource).unwrap();

	// 	// Should contain flattened deployment fields and type tag
	// 	assert!(json.contains("\"type\":\"deployment\""));
	// 	assert!(json.contains("\"name\":\"test-app\""));
	// 	// Should not contain nested "data" field due to #[serde(flatten)]
	// 	assert!(!json.contains("\"data\""));

	// 	let deserialized: IaacResource = serde_json::from_str(&json).unwrap();
	// 	assert_eq!(resource, deserialized);
	// }

	// #[test]
	// fn test_oneormore_dependency_serialization() {
	// 	// Test single dependency serialization (OneOrMore::One)
	// 	let single_dep = OneOrMore::One(Dependency {
	// 		resource: None,
	// 		identifier: Either::Right("single-dep".to_string()),
	// 	});

	// 	let json = serde_json::to_string(&single_dep).unwrap();
	// 	let deserialized: OneOrMore<Dependency> =
	// serde_json::from_str(&json).unwrap(); 	assert_eq!(single_dep,
	// deserialized);

	// 	// Test multiple dependencies serialization (OneOrMore::Multiple)
	// 	let multiple_deps = OneOrMore::Multiple(vec![
	// 		Dependency {
	// 			resource: Some(ResourceType::Deployment),
	// 			identifier: Either::Right("dep1".to_string()),
	// 		},
	// 		Dependency {
	// 			resource: None,
	// 			identifier: Either::Right("dep2".to_string()),
	// 		},
	// 	]);

	// 	let json = serde_json::to_string(&multiple_deps).unwrap();
	// 	let deserialized: OneOrMore<Dependency> =
	// serde_json::from_str(&json).unwrap(); 	assert_eq!(multiple_deps,
	// deserialized); }

	// #[test]
	// fn test_iaac_resource_default_trait() {
	// 	// Test that MaybeExternallySourced implements Default correctly
	// 	let default_string: MaybeExternallySourced<String> =
	// MaybeExternallySourced::default(); 	assert_eq!(
	// 		default_string,
	// 		MaybeExternallySourced::Value(String::default())
	// 	);

	// 	let default_int: MaybeExternallySourced<i32> =
	// MaybeExternallySourced::default(); 	assert_eq!(default_int,
	// MaybeExternallySourced::Value(0));

	// 	let default_bool: MaybeExternallySourced<bool> =
	// MaybeExternallySourced::default(); 	assert_eq!(default_bool,
	// MaybeExternallySourced::Value(false)); }

	// #[test]
	// fn test_from_trait_implementation() {
	// 	// Test From<T> trait implementation for MaybeExternallySourced
	// 	let from_string: MaybeExternallySourced<String> =
	// "test".to_string().into(); 	assert_eq!(
	// 		from_string,
	// 		MaybeExternallySourced::Value("test".to_string())
	// 	);

	// 	let from_int: MaybeExternallySourced<i32> = 42.into();
	// 	assert_eq!(from_int, MaybeExternallySourced::Value(42));

	// 	let from_bool: MaybeExternallySourced<bool> = true.into();
	// 	assert_eq!(from_bool, MaybeExternallySourced::Value(true));
	// }

	// #[test]
	// fn test_complex_nested_serialization() {
	// 	// Test complex nested structure with multiple types
	// 	let uuid = Uuid::new_v4();
	// 	let deployment = IaacDeployment {
	// 		name: MaybeExternallySourced::FromEnvironment {
	// 			from_env: "APP_NAME".to_string(),
	// 		},
	// 	};

	// 	let resource = IaacResource {
	// 		data: IaacResourceData::Deployment(deployment),
	// 		depends_on: Some(OneOrMore::Multiple(vec![
	// 			Dependency {
	// 				resource: Some(ResourceType::Deployment),
	// 				identifier: Either::Left(uuid),
	// 			},
	// 			Dependency {
	// 				resource: None,
	// 				identifier: Either::Right("database-service".to_string()),
	// 			},
	// 		])),
	// 	};

	// 	let json = serde_json::to_string(&resource).unwrap();
	// 	let deserialized: IaacResource = serde_json::from_str(&json).unwrap();
	// 	assert_eq!(resource, deserialized);

	// 	// Verify specific JSON structure
	// 	assert!(json.contains("\"type\":\"deployment\""));
	// 	assert!(json.contains("\"from_env\":\"APP_NAME\""));
	// 	assert!(json.contains("\"depends_on\""));
	// 	assert!(json.contains(&uuid.to_string()));
	// 	assert!(json.contains("\"database-service\""));
	// }

	// #[test]
	// fn test_malformed_json_error_cases() {
	// 	// Test invalid MaybeExternallySourced structure
	// 	let invalid_mes_json = r#"{"invalid_field": "value"}"#;
	// 	let result: Result<MaybeExternallySourced<String>, _> =
	// 		serde_json::from_str(invalid_mes_json);
	// 	assert!(result.is_err());

	// 	// Test invalid Dependency structure
	// 	let invalid_dep_json = r#"{"resource": "invalid_type", "identifier":
	// "test"}"#; 	let result: Result<Dependency, _> =
	// serde_json::from_str(invalid_dep_json); 	assert!(result.is_err());

	// 	// Test invalid IaacResourceData type
	// 	let invalid_resource_json = r#"{"type": "invalid_type", "name":
	// "test"}"#; 	let result: Result<IaacResourceData, _> =
	// serde_json::from_str(invalid_resource_json); 	assert!(result.is_err());

	// 	// Test missing required fields
	// 	let missing_type_json = r#"{"name": "test"}"#;
	// 	let result: Result<IaacResourceData, _> =
	// serde_json::from_str(missing_type_json); 	assert!(result.is_err());
	// }

	// #[test]
	// fn test_serde_error_handling() {
	// 	// Test invalid JSON for MaybeExternallySourced
	// 	assert_de_tokens_error::<MaybeExternallySourced<String>>(
	// 		&[
	// 			Token::Map { len: Some(1) },
	// 			Token::Str("invalid"),
	// 			Token::Str("value"),
	// 			Token::MapEnd,
	// 		],
	// 		"invalid type: map, expected a string or struct FromEnvironment",
	// 	);

	// 	// Test invalid Either variant
	// 	assert_de_tokens_error::<Dependency>(
	// 		&[
	// 			Token::Struct {
	// 				name: "Dependency",
	// 				len: 2,
	// 			},
	// 			Token::Str("resource"),
	// 			Token::None,
	// 			Token::Str("identifier"),
	// 			Token::NewtypeVariant {
	// 				name: "Either",
	// 				variant: "Invalid",
	// 			},
	// 			Token::Str("test"),
	// 			Token::StructEnd,
	// 		],
	// 		"unknown variant `Invalid`, expected `Left` or `Right`",
	// 	);
	// }

	// #[test]
	// fn test_iaac_resource_comprehensive_tokens() {
	// 	// Test complete IaacResource structure with serde_test
	// 	let deployment = IaacDeployment {
	// 		name: MaybeExternallySourced::Value("test-deployment".to_string()),
	// 	};

	// 	let dependency = Dependency {
	// 		resource: Some(ResourceType::Deployment),
	// 		identifier: Either::Right("other-deployment".to_string()),
	// 	};

	// 	let resource = IaacResource {
	// 		data: IaacResourceData::Deployment(deployment),
	// 		depends_on: Some(OneOrMore::One(dependency)),
	// 	};

	// 	// Note: This would be complex to define all tokens for flattened struct
	// 	// Instead, we'll test JSON round-trip and key assertions
	// 	let json = serde_json::to_string(&resource).unwrap();
	// 	let deserialized: IaacResource = serde_json::from_str(&json).unwrap();
	// 	assert_eq!(resource, deserialized);

	// 	// Verify key serialization aspects
	// 	assert!(json.contains("\"type\":\"deployment\""));
	// 	assert!(json.contains("\"name\":\"test-deployment\""));
	// 	assert!(json.contains("\"depends_on\""));
	// 	assert!(json.contains("\"resource\":\"deployment\""));
	// 	assert!(json.contains("\"identifier\":\"other-deployment\""));
	// }

	// #[test]
	// fn test_one_or_more_serde_tokens() {
	// 	// Test OneOrMore::One variant
	// 	let single_dep = OneOrMore::One(Dependency {
	// 		resource: None,
	// 		identifier: Either::Right("single-dep".to_string()),
	// 	});

	// 	// OneOrMore uses untagged enum, so it serializes as the inner type
	// 	let json = serde_json::to_string(&single_dep).unwrap();
	// 	let deserialized: OneOrMore<Dependency> =
	// serde_json::from_str(&json).unwrap(); 	assert_eq!(single_dep,
	// deserialized);

	// 	// Test OneOrMore::Multiple variant
	// 	let multiple_deps = OneOrMore::Multiple(vec![
	// 		Dependency {
	// 			resource: Some(ResourceType::Deployment),
	// 			identifier: Either::Right("dep1".to_string()),
	// 		},
	// 		Dependency {
	// 			resource: None,
	// 			identifier: Either::Right("dep2".to_string()),
	// 		},
	// 	]);

	// 	let json = serde_json::to_string(&multiple_deps).unwrap();
	// 	let deserialized: OneOrMore<Dependency> =
	// serde_json::from_str(&json).unwrap(); 	assert_eq!(multiple_deps,
	// deserialized); }

	// #[test]
	// fn test_edge_case_serialization() {
	// 	// Test empty string
	// 	let empty_string: MaybeExternallySourced<String> =
	// 		MaybeExternallySourced::Value("".to_string());
	// 	assert_tokens(&empty_string, &[Token::Str("")]);

	// 	// Test zero values
	// 	let zero_int: MaybeExternallySourced<i32> =
	// MaybeExternallySourced::Value(0); 	assert_tokens(&zero_int,
	// &[Token::I32(0)]);

	// 	// Test false boolean
	// 	let false_bool: MaybeExternallySourced<bool> =
	// MaybeExternallySourced::Value(false); 	assert_tokens(&false_bool,
	// &[Token::Bool(false)]);

	// 	// Test very long environment variable name
	// 	let long_env_name = "A".repeat(100);
	// 	let long_env: MaybeExternallySourced<String> =
	// MaybeExternallySourced::FromEnvironment { 		from_env:
	// long_env_name.clone(), 	};
	// 	assert_tokens(
	// 		&long_env,
	// 		&[
	// 			Token::Struct {
	// 				name: "MaybeExternallySourced",
	// 				len: 1,
	// 			},
	// 			Token::Str("from_env"),
	// 			Token::Str(long_env_name),
	// 			Token::StructEnd,
	// 		],
	// 	);
	// }
}
