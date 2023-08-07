use std::{fmt::Display, ops::Deref, str::FromStr};

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ResourceType {
	Personal,
	Business,
}

impl ResourceType {
	pub fn is_personal(&self) -> bool {
		matches!(self, Self::Personal)
	}

	pub fn is_business(&self) -> bool {
		matches!(self, Self::Business)
	}
}

impl Display for ResourceType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ResourceType::Personal => write!(f, "personal"),
			ResourceType::Business => write!(f, "business"),
		}
	}
}

impl FromStr for ResourceType {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = s.to_lowercase();
		match s.as_str() {
			"personal" => Ok(Self::Personal),
			"business" => Ok(Self::Business),
			_ => Err(s),
		}
	}
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub struct Personal;
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub struct Business;

impl<'de> Deserialize<'de> for Personal {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let value = String::deserialize(deserializer)?;

		if value == "personal" {
			Ok(Personal)
		} else {
			Err(D::Error::custom("str is not `personal`"))
		}
	}
}

impl<'de> Deserialize<'de> for Business {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let value = String::deserialize(deserializer)?;

		if value == "business" {
			Ok(Business)
		} else {
			Err(D::Error::custom("str is not `business`"))
		}
	}
}

impl Serialize for Personal {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str("personal")
	}
}

impl Serialize for Business {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str("business")
	}
}

impl From<Personal> for ResourceType {
	fn from(_: Personal) -> Self {
		Self::Personal
	}
}

impl From<Business> for ResourceType {
	fn from(_: Business) -> Self {
		Self::Business
	}
}

impl Deref for Personal {
	type Target = ResourceType;

	fn deref(&self) -> &Self::Target {
		&ResourceType::Personal
	}
}

impl Deref for Business {
	type Target = ResourceType;

	fn deref(&self) -> &Self::Target {
		&ResourceType::Business
	}
}

impl AsRef<ResourceType> for Personal {
	fn as_ref(&self) -> &ResourceType {
		&ResourceType::Personal
	}
}

impl AsRef<ResourceType> for Business {
	fn as_ref(&self) -> &ResourceType {
		&ResourceType::Business
	}
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_tokens, Token};

	use super::{Business, Personal, ResourceType};

	#[test]
	fn assert_resource_types() {
		assert_tokens(
			&ResourceType::Business,
			&[
				Token::Enum {
					name: "ResourceType",
				},
				Token::Str("business"),
				Token::Unit,
			],
		);

		assert_tokens(
			&ResourceType::Personal,
			&[
				Token::Enum {
					name: "ResourceType",
				},
				Token::Str("personal"),
				Token::Unit,
			],
		);
	}

	#[test]
	fn assert_personal_types() {
		assert_tokens(&Personal, &[Token::Str("personal")]);
	}

	#[test]
	fn assert_business_types() {
		assert_tokens(&Business, &[Token::Str("business")]);
	}
}
