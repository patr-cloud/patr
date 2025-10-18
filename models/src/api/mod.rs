#![allow(clippy::missing_docs_in_private_items)]

/// All auth related endpoints, including OAuth
pub mod auth;
/// All endpoints that relate to a user and their data
pub mod user;
/// All endpoints that can be performed on a workspace
pub mod workspace;

/// Contains the trait that is used to represent all the data that will be sent
/// to an endpoint in the API. This trait is implemented for all the endpoints
/// in the API. Since it is a large trait, a helper macro is provided to
/// generate a request. See: [`macros::declare_api_endpoint`]
mod api_endpoint;
/// The endpoint to get the version of the API
mod get_version;

use std::ops::Deref;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use self::{api_endpoint::*, get_version::*};
use crate::prelude::*;

/// A wrapper for any type that contains an ID.
///
/// This is used to return data from the API that contains the ID of the object.
/// For example, when listing all deployments, the API will return a list of
/// `WithId<Deployment>`. This means that the `Deployment` struct should not
/// contain the ID field, or it will panic. The struct contained in the `WithId`
/// struct can be reused in multiple places.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash, JsonSchema)]
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
	/// Create a new [`WithId`] struct with the given Id and data. This helps
	/// instantiate the struct with the data and Id provided as parameters.
	pub fn new(id: impl Into<Uuid>, data: T) -> Self {
		Self {
			id: id.into(),
			data,
		}
	}
}

impl<T> Deref for WithId<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.data
	}
}

impl<ID> From<ID> for WithId<()>
where
	ID: Into<Uuid>,
{
	fn from(id: ID) -> Self {
		Self::new(id.into(), ())
	}
}

impl<T> ts_rs::TS for WithId<T> {
	type OptionInnerType = Self;
	type WithoutGenerics = WithId<()>;

	fn decl() -> String {
		format!(
			r#"
			type WithId<T> = {{
				id: string;
			}} & T;
			"#
		)
	}

	fn decl_concrete() -> String {
		format!(
			r#"
			type WithId<{}> = {{
				id: string;
			}} & {};
			"#,
			std::any::type_name::<T>(),
			std::any::type_name::<T>()
		)
	}

	fn name() -> String {
		format!("WithId<{}>", std::any::type_name::<T>())
	}

	fn inline() -> String {
		format!(
			r#"
			{{
				id: string;
			}} & {}
			"#,
			std::any::type_name::<T>()
		)
	}

	fn inline_flattened() -> String {
		format!(
			r#"
			{{
				id: string;
			}} & {}
			"#,
			std::any::type_name::<T>()
		)
	}
}

#[cfg(test)]
mod test {
	use serde_test::{Token, assert_tokens};

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
