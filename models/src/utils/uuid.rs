use std::fmt::Display;

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Uuid(uuid::Uuid);

impl Uuid {
	pub fn new_v4() -> Self {
		Self(uuid::Uuid::new_v4())
	}

	pub const fn nil() -> Self {
		Self(uuid::Uuid::nil())
	}

	pub fn parse_str(input: &str) -> Result<Self, uuid::Error> {
		uuid::Uuid::try_parse(input).map(Self)
	}

	pub const fn is_nil(&self) -> bool {
		self.0.is_nil()
	}

	pub const fn as_u128(&self) -> u128 {
		self.0.as_u128()
	}

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
		let string: &str = Deserialize::deserialize(deserializer)?;
		hex::decode_to_slice(string, &mut buffer)
			.map_err(Error::custom)?;
		Ok(Self(uuid::Uuid::from_bytes(buffer)))
	}
}
