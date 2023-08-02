use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::Address;
use crate::{utils::Uuid, ApiRequest};

#[derive(
	Eq,
	Ord,
	Hash,
	Debug,
	Clone,
	Default,
	TypedPath,
	PartialEq,
	Serialize,
	PartialOrd,
	Deserialize,
)]
#[typed_path("/workspace/:workspace_id/billing/billing-address")]
pub struct AddBillingAddressPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddBillingAddressRequest {
	pub address_details: Address,
}

impl ApiRequest for AddBillingAddressRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = AddBillingAddressPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::AddBillingAddressRequest;
	use crate::{models::workspace::billing::Address, ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&AddBillingAddressRequest {
				address_details: Address {
					first_name: String::from("John"),
					last_name: String::from("Patr"),
					address_line_1: "221B Baker St, Marylebone".to_string(),
					address_line_2: None,
					address_line_3: None,
					city: "London".to_string(),
					state: "Lincolnshire".to_string(),
					zip: "NW1 6XE".to_string(),
					country: "UK".to_string(),
				},
			},
			&[
				Token::Struct {
					name: "AddBillingAddressRequest",
					len: 1,
				},
				Token::Str("addressDetails"),
				Token::Struct {
					name: "Address",
					len: 7,
				},
				Token::Str("firstName"),
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Str("Patr"),
				Token::Str("addressLine1"),
				Token::Str("221B Baker St, Marylebone"),
				Token::Str("city"),
				Token::Str("London"),
				Token::Str("state"),
				Token::Str("Lincolnshire"),
				Token::Str("zip"),
				Token::Str("NW1 6XE"),
				Token::Str("country"),
				Token::Str("UK"),
				Token::StructEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_all_request_types() {
		assert_tokens(
			&AddBillingAddressRequest {
				address_details: Address {
					first_name: "john".to_string(),
					last_name: "patr".to_string(),
					address_line_1: "221B,".to_string(),
					address_line_2: Some("Baker St".to_string()),
					address_line_3: Some("Marylebone".to_string()),
					city: "London".to_string(),
					state: "Lincolnshire".to_string(),
					zip: "NW1 6XE".to_string(),
					country: "UK".to_string(),
				},
			},
			&[
				Token::Struct {
					name: "AddBillingAddressRequest",
					len: 1,
				},
				Token::Str("addressDetails"),
				Token::Struct {
					name: "Address",
					len: 9,
				},
				Token::Str("firstName"),
				Token::Str("john"),
				Token::Str("lastName"),
				Token::Str("patr"),
				Token::Str("addressLine1"),
				Token::Str("221B,"),
				Token::Str("addressLine2"),
				Token::Some,
				Token::Str("Baker St"),
				Token::Str("addressLine3"),
				Token::Some,
				Token::Str("Marylebone"),
				Token::Str("city"),
				Token::Str("London"),
				Token::Str("state"),
				Token::Str("Lincolnshire"),
				Token::Str("zip"),
				Token::Str("NW1 6XE"),
				Token::Str("country"),
				Token::Str("UK"),
				Token::StructEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<AddBillingAddressRequest as ApiRequest>::Response>(
			(),
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(()),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::MapEnd,
			],
		);
	}
}
