/// All auth related endpoints, including OAuth
pub mod auth;
// pub mod ci;
// pub mod user;
pub mod workspace;
mod get_version;

use serde::{Deserialize, Serialize};

pub use self::get_version::*;
use crate::prelude::*;

/// A wrapper for any type that contains an ID. This is used to return data from
/// the API that contains the ID of the object. For example, when listing all
/// deployments, the API will return a list of `WithId<Deployment>`. This means
/// that the `Deployment` struct should not contain the ID field, or it will
/// panic. The struct contained in the `WithId` struct can be reused in multiple
/// places.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "camelCase")]
pub struct WithId<T> {
	/// The ID of the object. For example, in case of a deployment, this would
	/// be the deploymentId, and in case of a workspace, this would be the
	/// workspaceId and so on.
	pub id: Uuid,
	/// The data of the object. This can be any type that contains additional
	/// data that will be flattened. Note: This should not contain an Id field.
	#[serde(flatten)]
	pub data: T,
}

impl<T> WithId<T> {
	/// Create a new `WithId` struct with the given Id and data. This helps
	/// instantiate the struct with the data and Id provided as parameters.
	pub fn new(id: Uuid, data: T) -> Self {
		Self { id, data }
	}
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::WithId;
	use crate::prelude::Uuid;

	#[test]
	pub fn test_with_id_empty() {
		assert_tokens(
			&WithId::new(Uuid::nil(), ()),
			&[
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("00000000000000000000000000000000"),
				Token::MapEnd,
			],
		);
	}
}
