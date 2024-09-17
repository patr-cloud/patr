use std::{borrow::Cow, fmt::Display, str::FromStr};

use schemars::JsonSchema;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use time::{Duration, OffsetDateTime};

use super::constants;

/// A wrapper around [`uuid::Uuid`] that implements [`serde::Serialize`] and
/// [`serde::Deserialize`], but specifically only in the form of a hex string.
///
/// Any other format will be rejected. In API requests and response, this should
/// be used instead of [`uuid::Uuid`]. Ideally, this is the struct that should
/// be imported and [`uuid`] should not be added as a dependency at all. This
/// would prevent wrong UUIDs from being sent or received.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, JsonSchema)]
pub struct Uuid(uuid::Uuid);

impl Uuid {
	/// Create a new v1 (sequencially generated) [`Uuid`] with the current
	/// timestamp
	pub fn now_v1() -> Self {
		Self(uuid::Uuid::now_v1(&constants::UUID_NODE_ID))
	}

	/// Creates a new v4 (randomly generated) [`Uuid`]
	pub fn new_v4() -> Self {
		Self(uuid::Uuid::new_v4())
	}

	/// Creates a new nil [`Uuid`] (all zeroes).
	pub const fn nil() -> Self {
		Self(uuid::Uuid::nil())
	}

	/// Parses a [`Uuid`] from a string of hexadecimal digits with optional
	/// hyphens.
	pub fn parse_str(input: &str) -> Result<Self, uuid::Error> {
		uuid::Uuid::try_parse(input).map(Self)
	}

	/// A helper function to check if the [`Uuid`] is nil (all zeroes).
	pub const fn is_nil(&self) -> bool {
		self.0.is_nil()
	}

	/// Gets the timestamp of the Uuid if it is a v1 Uuid.
	pub fn get_timestamp(&self) -> Option<OffsetDateTime> {
		self.0.get_timestamp().and_then(|ts| {
			let (secs, nanos) = ts.to_unix();
			Some(
				OffsetDateTime::from_unix_timestamp(secs.try_into().unwrap_or_default()).ok()? +
					Duration::nanoseconds(nanos.into()),
			)
		})
	}

	/// Returns a 128-bit number representing the [`Uuid`].
	pub const fn as_u128(&self) -> u128 {
		self.0.as_u128()
	}

	/// Returns a 16-element byte array representing the [`Uuid`].
	pub const fn as_bytes(&self) -> &[u8; 16] {
		self.0.as_bytes()
	}
}

impl Display for Uuid {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0.simple())
	}
}

impl Serialize for Uuid {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&self.to_string())
	}
}

impl<'de> Deserialize<'de> for Uuid {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let mut buffer = [0u8; 16];
		let string: Cow<'de, str> = Deserialize::deserialize(deserializer)?;
		hex::decode_to_slice(string.as_ref(), &mut buffer).map_err(Error::custom)?;
		Ok(Self(uuid::Uuid::from_bytes(buffer)))
	}
}

impl From<Uuid> for uuid::Uuid {
	fn from(Uuid(value): Uuid) -> Self {
		value
	}
}

impl From<uuid::Uuid> for Uuid {
	fn from(uuid: uuid::Uuid) -> Self {
		Self(uuid)
	}
}

impl FromStr for Uuid {
	type Err = uuid::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Self::parse_str(s)
	}
}

// For backend
#[cfg(not(target_arch = "wasm32"))]
impl sqlx::Type<sqlx::Sqlite> for Uuid {
	fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
		<uuid::fmt::Simple as sqlx::Type<sqlx::Sqlite>>::type_info()
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl sqlx::Type<sqlx::Postgres> for Uuid {
	fn type_info() -> <sqlx::Postgres as sqlx::Database>::TypeInfo {
		<uuid::Uuid as sqlx::Type<sqlx::Postgres>>::type_info()
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl<'a> sqlx::Encode<'a, sqlx::Sqlite> for Uuid
where
	uuid::Uuid: sqlx::Encode<'a, sqlx::Sqlite>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <sqlx::Sqlite as sqlx::Database>::ArgumentBuffer<'a>,
	) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
		self.0.simple().encode_by_ref(buf)
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl<'a> sqlx::Encode<'a, sqlx::Postgres> for Uuid
where
	uuid::Uuid: sqlx::Encode<'a, sqlx::Postgres>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <sqlx::Postgres as sqlx::Database>::ArgumentBuffer<'a>,
	) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
		self.0.encode_by_ref(buf)
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl<'a> sqlx::Decode<'a, sqlx::Sqlite> for Uuid
where
	uuid::fmt::Simple: sqlx::Decode<'a, sqlx::Sqlite>,
{
	fn decode(
		value: <sqlx::Sqlite as sqlx::Database>::ValueRef<'a>,
	) -> Result<Self, sqlx::error::BoxDynError> {
		uuid::fmt::Simple::decode(value)
			.map(uuid::fmt::Simple::into_uuid)
			.map(Self)
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl<'a> sqlx::Decode<'a, sqlx::Postgres> for Uuid
where
	uuid::Uuid: sqlx::Decode<'a, sqlx::Postgres>,
{
	fn decode(
		value: <sqlx::Postgres as sqlx::Database>::ValueRef<'a>,
	) -> Result<Self, sqlx::error::BoxDynError> {
		uuid::Uuid::decode(value).map(Self)
	}
}
