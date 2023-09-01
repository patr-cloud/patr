use std::ops::Deref;

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

/// A type that can be used to represent a constant `false` boolean.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct False;

/// A type that can be used to represent a constant `true` boolean.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct True;

impl<'de> Deserialize<'de> for False {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let value = bool::deserialize(deserializer)?;

		if !value {
			Ok(False)
		} else {
			Err(D::Error::custom("bool is not false"))
		}
	}
}

impl<'de> Deserialize<'de> for True {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let value = bool::deserialize(deserializer)?;

		if value {
			Ok(True)
		} else {
			Err(D::Error::custom("bool is not true"))
		}
	}
}

impl Serialize for False {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_bool(false)
	}
}

impl Serialize for True {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_bool(true)
	}
}

impl From<True> for bool {
	fn from(_: True) -> Self {
		true
	}
}

impl From<False> for bool {
	fn from(_: False) -> Self {
		false
	}
}

impl Deref for True {
	type Target = bool;

	fn deref(&self) -> &Self::Target {
		&true
	}
}

impl Deref for False {
	type Target = bool;

	fn deref(&self) -> &Self::Target {
		&false
	}
}

impl AsRef<bool> for True {
	fn as_ref(&self) -> &bool {
		&true
	}
}

impl AsRef<bool> for False {
	fn as_ref(&self) -> &bool {
		&false
	}
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_tokens, Token};

	use super::{False, True};

	#[test]
	fn assert_true_types() {
		assert_tokens(&True, &[Token::Bool(true)]);
	}

	#[test]
	fn assert_false_types() {
		assert_tokens(&False, &[Token::Bool(false)]);
	}
}
