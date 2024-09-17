use std::{
	fmt::Display,
	ops::{Deref, DerefMut},
};

use base64::prelude::*;
use schemars::JsonSchema;
use serde::{de::Error, Serialize};

/// A wrapper around a `Vec<u8>` that implements `Display` and `Serialize` to
/// encode the data as base64. Mostly used for config mount values.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
pub struct Base64String {
	/// The data that is being wrapped.
	data: Vec<u8>,
}

impl Base64String {
	/// Convert the `Base64String` into a `Vec<u8>`.
	pub fn into_vec(self) -> Vec<u8> {
		self.data
	}
}

impl From<Vec<u8>> for Base64String {
	fn from(data: Vec<u8>) -> Self {
		Base64String { data }
	}
}

impl From<&[u8]> for Base64String {
	fn from(value: &[u8]) -> Self {
		Base64String {
			data: value.to_vec(),
		}
	}
}

impl From<Base64String> for Vec<u8> {
	fn from(value: Base64String) -> Vec<u8> {
		value.data
	}
}

impl Deref for Base64String {
	type Target = Vec<u8>;

	fn deref(&self) -> &Self::Target {
		&self.data
	}
}

impl DerefMut for Base64String {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.data
	}
}

impl AsRef<Vec<u8>> for Base64String {
	fn as_ref(&self) -> &Vec<u8> {
		&self.data
	}
}

impl AsRef<[u8]> for Base64String {
	fn as_ref(&self) -> &[u8] {
		&self.data
	}
}

impl Display for Base64String {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", BASE64_STANDARD.encode(&self.data))
	}
}

impl Serialize for Base64String {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(BASE64_STANDARD.encode(&self.data).as_str())
	}
}

impl<'de> serde::Deserialize<'de> for Base64String {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let string = String::deserialize(deserializer)?;
		BASE64_STANDARD
			.decode(&string)
			.map_err(|_| Error::custom(format!("unable to decode {} as base64", string)))
			.map(|data| Base64String { data })
	}
}
