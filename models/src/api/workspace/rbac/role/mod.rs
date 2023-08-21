mod create_new_role;
mod delete_role;
mod get_role_info;
mod list_all_roles;
mod list_users_for_role;
mod update_role;

use serde::{Deserialize, Serialize};

pub use self::{
	create_new_role::*,
	delete_role::*,
	get_role_info::*,
	list_all_roles::*,
	list_users_for_role::*,
	update_role::*,
};
use crate::utils::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Role {
	pub id: Uuid,
	pub name: String,
	#[serde(default, skip_serializing_if = "String::is_empty")]
	pub description: String,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::Role;
	use crate::utils::Uuid;

	#[test]
	fn assert_role_types() {
		assert_tokens(
			&Role {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				name: "Software Developer".to_string(),
				description: String::new(),
			},
			&[
				Token::Struct {
					name: "Role",
					len: 2,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("Software Developer"),
				Token::StructEnd,
			],
		);
	}
}
