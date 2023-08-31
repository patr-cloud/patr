use models::RequestUserData;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessTokenData {
	pub iss: String,
	pub aud: String,
	#[serde(with = "datetime_as_seconds")]
	pub iat: OffsetDateTime,
	pub typ: String,
	#[serde(with = "datetime_as_seconds")]
	pub exp: OffsetDateTime,
	pub user: RequestUserData,
}

mod datetime_as_seconds {
	use serde::{de::Error, Deserialize, Deserializer, Serializer};
	use time::OffsetDateTime;

	pub fn serialize<S>(value: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_i64(value.unix_timestamp())
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
	where
		D: Deserializer<'de>,
	{
		OffsetDateTime::from_unix_timestamp(i64::deserialize(deserializer)?).map_err(Error::custom)
	}
}
