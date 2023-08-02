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
pub struct GetBillingAddressPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetBillingAddressRequest;

impl ApiRequest for GetBillingAddressRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetBillingAddressPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GetBillingAddressResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetBillingAddressResponse {
	pub address: Option<Address>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{GetBillingAddressRequest, GetBillingAddressResponse};
	use crate::{models::workspace::billing::Address, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetBillingAddressRequest,
			&[Token::UnitStruct {
				name: "GetBillingAddressRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetBillingAddressResponse {
				address: Some(Address {
					first_name: String::from("John"),
					last_name: String::from("Patr"),
					address_line_1: "221B Baker St, Marylebone".to_string(),
					address_line_2: None,
					address_line_3: None,
					city: "London".to_string(),
					state: "Lincolnshire".to_string(),
					zip: "NW1 6XE".to_string(),
					country: "United Kingdom".to_string(),
				}),
			},
			&[
				Token::Struct {
					name: "GetBillingAddressResponse",
					len: 1,
				},
				Token::Str("address"),
				Token::Some,
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
				Token::Str("United Kingdom"),
				Token::StructEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetBillingAddressResponse {
				address: Some(Address {
					first_name: String::from("John"),
					last_name: String::from("Patr"),
					address_line_1: "221B Baker St, Marylebone".to_string(),
					address_line_2: None,
					address_line_3: None,
					city: "London".to_string(),
					state: "Lincolnshire".to_string(),
					zip: "NW1 6XE".to_string(),
					country: "United Kingdom".to_string(),
				}),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("address"),
				Token::Some,
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
				Token::Str("United Kingdom"),
				Token::StructEnd,
				Token::MapEnd,
			],
		);
	}
}
