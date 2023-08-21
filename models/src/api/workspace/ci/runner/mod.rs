mod create_runner;
mod delete_runner;
mod get_runner_info;
mod list_runner;
mod list_runner_build_history;
mod update_runner;

use serde::{Deserialize, Serialize};

pub use self::{
	create_runner::*,
	delete_runner::*,
	get_runner_info::*,
	list_runner::*,
	list_runner_build_history::*,
	update_runner::*,
};
use crate::utils::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Runner {
	pub id: Uuid,
	pub name: String,
	pub region_id: Uuid,
	pub build_machine_type_id: Uuid,
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_tokens, Token};

	use super::*;

	#[test]
	fn assert_ci_runner_type() {
		assert_tokens(
			&Runner {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				name: "runner name".into(),
				region_id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30869")
					.unwrap(),
				build_machine_type_id: Uuid::parse_str(
					"2aef18631ded45eb9170dc2166b30869",
				)
				.unwrap(),
			},
			&[
				Token::Struct {
					name: "Runner",
					len: 4,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("runner name"),
				Token::Str("regionId"),
				Token::Str("2aef18631ded45eb9170dc2166b30869"),
				Token::Str("buildMachineTypeId"),
				Token::Str("2aef18631ded45eb9170dc2166b30869"),
				Token::StructEnd,
			],
		)
	}
}
