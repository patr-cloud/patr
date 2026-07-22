use std::{
	fmt::Display,
	ops::{Deref, DerefMut},
};

use base64::prelude::*;
use schemars::JsonSchema;
use serde::{Serialize, de::Error};

/// A wrapper around a `Vec<u8>` that implements `Display` and `Serialize` to
/// encode the data as base64. Mostly used for config mount values.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, JsonSchema, ts_rs::TS)]
#[ts(as = "String")]
pub struct Base64String {
	/// The data that is being wrapped.
	data: Vec<u8>,
}

impl Base64String {
	/// Convert the `Base64String` into a `Vec<u8>`.
	#[must_use]
	pub fn into_vec(self) -> Vec<u8> {
		self.data
	}

	/// Create a new `Base64String` from a `String`.
	#[must_use]
	pub fn from_string(data: String) -> Self {
		Self {
			data: data.into_bytes(),
		}
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
			.map_err(|_| Error::custom(format!("unable to decode {string} as base64")))
			.map(|data| Base64String { data })
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The raw content used across the round-trip tests, and its canonical
	/// base64 encoding.
	const RAW: &[u8] = br#"{"installed":{"client_id":"x"}}"#;
	const ENCODED: &str = "eyJpbnN0YWxsZWQiOnsiY2xpZW50X2lkIjoieCJ9fQ==";

	/// `From<Vec<u8>>` and `from_string` must store the bytes verbatim — no
	/// encoding. Wrapping raw content and then reading it back (deref /
	/// `into_vec`) yields exactly the input. This is the invariant the
	/// config-mount double-encode bug violated by treating the wrapped bytes as
	/// their base64 string form.
	#[test]
	fn from_bytes_stores_verbatim() {
		let wrapped = Base64String::from(RAW.to_vec());
		assert_eq!(&*wrapped, RAW);
		assert_eq!(wrapped.as_slice(), RAW);
		assert_eq!(wrapped.into_vec(), RAW.to_vec());

		let from_str = Base64String::from_string(String::from("hello"));
		assert_eq!(from_str.into_vec(), b"hello".to_vec());
	}

	/// `Display` and `Serialize` are the *encoding* side: both emit base64 of
	/// the stored bytes.
	#[test]
	fn display_and_serialize_base64_encode() {
		let wrapped = Base64String::from(RAW.to_vec());
		assert_eq!(wrapped.to_string(), ENCODED);
		assert_eq!(
			serde_json::to_string(&wrapped).unwrap(),
			format!("\"{ENCODED}\"")
		);
	}

	/// `Deserialize` decodes base64 back into the raw bytes.
	#[test]
	fn deserialize_base64_decodes() {
		let wrapped: Base64String = serde_json::from_str(&format!("\"{ENCODED}\"")).unwrap();
		assert_eq!(wrapped.as_slice(), RAW);
	}

	/// Serialize → Deserialize is the identity on the raw bytes (one encode,
	/// one decode) — proving a single, symmetric base64 layer on the wire.
	#[test]
	fn serialize_deserialize_round_trips() {
		let original = Base64String::from(RAW.to_vec());
		let json = serde_json::to_string(&original).unwrap();
		let back: Base64String = serde_json::from_str(&json).unwrap();
		assert_eq!(original, back);
		assert_eq!(back.as_slice(), RAW);
	}

	/// Non-base64 input fails to deserialize rather than silently mangling.
	#[test]
	fn deserialize_rejects_non_base64() {
		let result: Result<Base64String, _> = serde_json::from_str("\"not base64!!!\"");
		assert!(result.is_err());
	}
}
