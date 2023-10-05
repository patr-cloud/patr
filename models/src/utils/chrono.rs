use std::{
	convert::TryFrom,
	fmt::{Debug, Display},
	ops::{Deref, DerefMut},
	str::FromStr,
};

use chrono::{TimeZone, Utc};
use serde::{de::Error, Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTime<Tz>(pub chrono::DateTime<Tz>)
where
	Tz: TimeZone;

impl<Tz> Debug for DateTime<Tz>
where
	Tz: TimeZone,
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{:?}", self.0)
	}
}

impl Default for DateTime<Utc> {
	fn default() -> Self {
		let inner = chrono::DateTime::<Utc>::default();
		Self(inner)
	}
}

impl<Tz> From<chrono::DateTime<Tz>> for DateTime<Tz>
where
	Tz: TimeZone,
{
	fn from(inner: chrono::DateTime<Tz>) -> Self {
		Self(inner)
	}
}

impl<Tz> From<DateTime<Tz>> for chrono::DateTime<Tz>
where
	Tz: TimeZone,
{
	fn from(inner: DateTime<Tz>) -> Self {
		inner.0
	}
}

impl<Tz> Deref for DateTime<Tz>
where
	Tz: TimeZone,
{
	type Target = chrono::DateTime<Tz>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<Tz> DerefMut for DateTime<Tz>
where
	Tz: TimeZone,
{
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl<Tz> Serialize for DateTime<Tz>
where
	Tz: TimeZone,
	<Tz as TimeZone>::Offset: Display,
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(&self.0.to_rfc2822())
	}
}

impl<'de> Deserialize<'de> for DateTime<Utc> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let string = String::deserialize(deserializer)?;
		let inner = chrono::DateTime::parse_from_rfc2822(&string)
			.map(|inner| inner.with_timezone(&Utc))
			.map_err(Error::custom)?;
		Ok(Self(inner))
	}
}

impl<Tz> TryFrom<&str> for DateTime<Tz>
where
	Tz: TimeZone,
	chrono::DateTime<Tz>: FromStr,
{
	type Error = <chrono::DateTime<Tz> as FromStr>::Err;

	fn try_from(string: &str) -> Result<Self, Self::Error> {
		chrono::DateTime::from_str(string).map(Self)
	}
}

impl<Tz> FromStr for DateTime<Tz>
where
	Tz: TimeZone,
	chrono::DateTime<Tz>: FromStr,
{
	type Err = <chrono::DateTime<Tz> as FromStr>::Err;

	fn from_str(string: &str) -> Result<Self, Self::Err> {
		chrono::DateTime::from_str(string).map(Self)
	}
}

#[cfg(test)]
mod tests {
	use std::str::FromStr;

	use serde_test::{assert_tokens, Token};

	use super::DateTime;

	#[test]
	fn assert_date_time_types() {
		assert_tokens(
			&DateTime::from_str("2020-04-12 22:10:57+02:00").unwrap(),
			&[Token::Str("Sun, 12 Apr 2020 20:10:57 +0000")],
		);
	}
}
