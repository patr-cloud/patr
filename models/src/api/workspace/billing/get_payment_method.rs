use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::PaymentMethod;
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
#[typed_path("/workspace/:workspace_id/billing/payment-method")]
pub struct GetPaymentMethodPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetPaymentMethodRequest;

impl ApiRequest for GetPaymentMethodRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetPaymentMethodPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GetPaymentMethodResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetPaymentMethodResponse {
	pub list: Vec<PaymentMethod>,
}

#[cfg(test)]
mod test {
	use chrono::{TimeZone, Utc};
	use serde_test::{assert_tokens, Token};

	use super::{GetPaymentMethodRequest, GetPaymentMethodResponse};
	use crate::{
		models::workspace::billing::{Card, CardFundingType, PaymentMethod},
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetPaymentMethodRequest,
			&[Token::UnitStruct {
				name: "GetPaymentMethodRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetPaymentMethodResponse {
				list: vec![PaymentMethod {
					id: "pm_1LAWo42eZvKYlo2CQbcaFOOe".to_string(),
					customer: "cus_4QE41GDczMg5d5".to_string(),
					card: Some(Card {
						brand: "visa".to_string(),
						country: "US".to_string(),
						exp_month: 8,
						exp_year: 2023,
						funding: CardFundingType::Credit,
						last4: "4242".to_string(),
					}),
					created: Utc.timestamp_opt(0, 0).unwrap().into(),
				}],
			},
			&[
				Token::Struct {
					name: "GetPaymentMethodResponse",
					len: 1,
				},
				Token::Str("list"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "PaymentMethod",
					len: 4,
				},
				Token::Str("id"),
				Token::Str("pm_1LAWo42eZvKYlo2CQbcaFOOe"),
				Token::Str("customer"),
				Token::Str("cus_4QE41GDczMg5d5"),
				Token::Str("card"),
				Token::Some,
				Token::Struct {
					name: "Card",
					len: 6,
				},
				Token::Str("brand"),
				Token::Str("visa"),
				Token::Str("country"),
				Token::Str("US"),
				Token::Str("expMonth"),
				Token::U32(8),
				Token::Str("expYear"),
				Token::U32(2023),
				Token::Str("funding"),
				Token::UnitVariant {
					name: "CardFundingType",
					variant: "credit",
				},
				Token::Str("last4"),
				Token::Str("4242"),
				Token::StructEnd,
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetPaymentMethodResponse {
				list: vec![PaymentMethod {
					id: "pm_1LAWo42eZvKYlo2CQbcaFOOe".to_string(),
					customer: "cus_4QE41GDczMg5d5".to_string(),
					card: Some(Card {
						brand: "visa".to_string(),
						country: "US".to_string(),
						exp_month: 8,
						exp_year: 2023,
						funding: CardFundingType::Credit,
						last4: "4242".to_string(),
					}),
					created: Utc.timestamp_opt(0, 0).unwrap().into(),
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("list"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "PaymentMethod",
					len: 4,
				},
				Token::Str("id"),
				Token::Str("pm_1LAWo42eZvKYlo2CQbcaFOOe"),
				Token::Str("customer"),
				Token::Str("cus_4QE41GDczMg5d5"),
				Token::Str("card"),
				Token::Some,
				Token::Struct {
					name: "Card",
					len: 6,
				},
				Token::Str("brand"),
				Token::Str("visa"),
				Token::Str("country"),
				Token::Str("US"),
				Token::Str("expMonth"),
				Token::U32(8),
				Token::Str("expYear"),
				Token::U32(2023),
				Token::Str("funding"),
				Token::UnitVariant {
					name: "CardFundingType",
					variant: "credit",
				},
				Token::Str("last4"),
				Token::Str("4242"),
				Token::StructEnd,
				Token::Str("created"),
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
