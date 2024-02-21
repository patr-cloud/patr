use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

/// Represents a value that can be either one or many. This is useful for
/// serializing and deserializing JSON values that can be either a single
/// object or an array of objects.
///
/// The default implementation is [`OneOrMore::One(Default::default())`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OneOrMore<T> {
	/// A single value.
	One(T),
	/// A list of values.
	Multiple(Vec<T>),
}

impl<T> OneOrMore<T>
where
	T: PartialEq,
{
	/// If a single value is present, checks if the provided value is equal to
	/// it. If multiple values are present, checks if the provided value is
	/// present in the list.
	///
	/// This is mostly used in scenarios where we need to check if one of the
	/// values are present, regardless of whether it is a single value or
	/// present in a list.
	pub fn contains(&self, value: &T) -> bool {
		match self {
			Self::One(one) => one == value,
			Self::Multiple(many) => many.contains(value),
		}
	}
}

impl<T> PartialEq for OneOrMore<T>
where
	T: PartialEq,
{
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::One(lhs), Self::One(rhs)) => lhs == rhs,
			(Self::Multiple(lhs), Self::Multiple(rhs)) => lhs == rhs,
			(Self::One(one), Self::Multiple(many)) if many.len() == 1 => {
				many.first().map(|first| one == first).unwrap_or(false)
			}
			(Self::Multiple(many), Self::One(one)) if many.len() == 1 => {
				many.first().map(|first| one == first).unwrap_or(false)
			}
			_ => false,
		}
	}
}

impl<T> Eq for OneOrMore<T> where T: Eq {}

impl<T> Default for OneOrMore<T>
where
	T: Default,
{
	fn default() -> Self {
		Self::One(Default::default())
	}
}

impl<T> PartialOrd for OneOrMore<T>
where
	T: PartialOrd,
{
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		match (self, other) {
			(Self::One(lhs), Self::One(rhs)) => lhs.partial_cmp(rhs),
			(Self::Multiple(lhs), Self::Multiple(rhs)) => lhs.partial_cmp(rhs),
			(Self::One(one), Self::Multiple(many)) => many
				.first()
				.map(|first| one.partial_cmp(first))
				.unwrap_or(None),
			(Self::Multiple(many), Self::One(one)) => many
				.first()
				.map(|first| first.partial_cmp(one))
				.unwrap_or(None),
		}
	}
}

impl<T> Ord for OneOrMore<T>
where
	T: Ord,
{
	fn cmp(&self, other: &Self) -> Ordering {
		match (self, other) {
			(OneOrMore::One(a), OneOrMore::One(b)) => a.cmp(b),
			(OneOrMore::Multiple(a), OneOrMore::Multiple(b)) => a.cmp(b),
			(OneOrMore::One(a), OneOrMore::Multiple(b)) => Some(a).cmp(&b.first()),
			(OneOrMore::Multiple(a), OneOrMore::One(b)) => a.first().cmp(&Some(b)),
		}
	}
}

impl<T> IntoIterator for OneOrMore<T> {
	type IntoIter = std::vec::IntoIter<T>;
	type Item = T;

	fn into_iter(self) -> Self::IntoIter {
		match self {
			OneOrMore::One(t) => vec![t].into_iter(),
			OneOrMore::Multiple(v) => v.into_iter(),
		}
	}
}

impl<T> From<T> for OneOrMore<T> {
	fn from(value: T) -> Self {
		OneOrMore::One(value)
	}
}
