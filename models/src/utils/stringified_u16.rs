use std::{fmt::Display, num::ParseIntError, ops::Deref, str::FromStr};

use schemars::JsonSchema;
use serde::{
	Deserialize,
	Serialize,
	de::{Error, Visitor},
};

/// A wrapper around a `u16` that serializes and deserializes as a string.
/// Mostly used as keys in maps.
#[derive(Clone, Copy, Debug, PartialEq, Hash, Eq, PartialOrd, Ord, JsonSchema)]
pub struct StringifiedU16(u16);

impl StringifiedU16 {
	/// Create a new instance of the [`StringifiedU16`] with the given value.
	#[must_use]
	pub fn new(value: u16) -> Self {
		Self(value)
	}

	/// Get the value of the [`StringifiedU16`].
	#[must_use]
	pub fn value(&self) -> u16 {
		self.0
	}
}

impl Deref for StringifiedU16 {
	type Target = u16;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl From<u16> for StringifiedU16 {
	fn from(value: u16) -> Self {
		StringifiedU16(value)
	}
}

impl FromStr for StringifiedU16 {
	type Err = ParseIntError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		s.parse::<u16>().map(StringifiedU16)
	}
}

/// The serde visitor for [`StringifiedU16`].
struct StringifiedU16Visitor;

impl<'de> Visitor<'de> for StringifiedU16Visitor {
	type Value = StringifiedU16;

	fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
		formatter.write_str("a string or number representing a u16")
	}

	fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
	where
		E: Error,
	{
		let number = v.parse().map_err(Error::custom)?;
		Ok(StringifiedU16(number))
	}

	fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
	where
		E: Error,
	{
		u16::try_from(v)
			.map(StringifiedU16)
			.map_err(|_| Error::custom(format!("number {} is out of range for u16", v)))
	}

	fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
	where
		E: Error,
	{
		u16::try_from(v)
			.map(StringifiedU16)
			.map_err(|_| Error::custom(format!("number {} is out of range for u16", v)))
	}
}

impl<'de> Deserialize<'de> for StringifiedU16 {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		deserializer.deserialize_any(StringifiedU16Visitor)
	}
}

impl Serialize for StringifiedU16 {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(&self.0.to_string())
	}
}

impl Display for StringifiedU16 {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl AsRef<u16> for StringifiedU16 {
	fn as_ref(&self) -> &u16 {
		&self.0
	}
}

impl ts_rs::TS for StringifiedU16 {
	type OptionInnerType = <u16 as ts_rs::TS>::OptionInnerType;
	type WithoutGenerics = <u16 as ts_rs::TS>::WithoutGenerics;

	fn decl(config: &ts_rs::Config) -> String {
		<u16 as ts_rs::TS>::decl(config)
	}

	fn decl_concrete(config: &ts_rs::Config) -> String {
		<u16 as ts_rs::TS>::decl_concrete(config)
	}

	fn name(config: &ts_rs::Config) -> String {
		<u16 as ts_rs::TS>::name(config)
	}

	fn inline(config: &ts_rs::Config) -> String {
		<u16 as ts_rs::TS>::inline(config)
	}

	fn inline_flattened(config: &ts_rs::Config) -> String {
		<u16 as ts_rs::TS>::inline_flattened(config)
	}
}

#[cfg(test)]
mod tests {
	use serde_test::{Token, assert_tokens};

	use super::StringifiedU16;

	#[test]
	fn assert_stringified_u16_types() {
		assert_tokens(&StringifiedU16(25), &[Token::Str("25")])
	}
}
