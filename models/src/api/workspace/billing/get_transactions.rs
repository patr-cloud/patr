use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::Transaction;
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
#[typed_path("/workspace/:workspace_id/billing/transaction-history")]
pub struct GetTransactionHistoryPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetTransactionHistoryRequest;

impl ApiRequest for GetTransactionHistoryRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetTransactionHistoryPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GetTransactionHistoryResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetTransactionHistoryResponse {
	pub transactions: Vec<Transaction>,
}

#[cfg(test)]
mod test {
	use chrono::{TimeZone, Utc};
	use serde_test::{assert_tokens, Token};

	use super::{GetTransactionHistoryRequest, GetTransactionHistoryResponse};
	use crate::{
		models::workspace::billing::{
			PaymentStatus,
			Transaction,
			TransactionType,
		},
		utils::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetTransactionHistoryRequest,
			&[Token::UnitStruct {
				name: "GetTransactionHistoryRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetTransactionHistoryResponse {
				transactions: vec![Transaction {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					month: 5,
					amount: 6969,
					payment_intent_id: Some(
						"pm_6K95KhSGEPBh7GrIsWVB4pyV".to_string(),
					),
					date: Utc.timestamp_opt(1431648000, 0).unwrap().into(),
					workspace_id: Uuid::parse_str(
						"2aef18631ded45eb9170dc2166b30867",
					)
					.unwrap(),
					transaction_type: TransactionType::Payment,
					payment_status: PaymentStatus::Success,
					description: None,
				}],
			},
			&[
				Token::Struct {
					name: "GetTransactionHistoryResponse",
					len: 1,
				},
				Token::Str("transactions"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "Transaction",
					len: 9,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("month"),
				Token::I32(5),
				Token::Str("amount"),
				Token::U64(6969),
				Token::Str("paymentIntentId"),
				Token::Some,
				Token::Str("pm_6K95KhSGEPBh7GrIsWVB4pyV"),
				Token::Str("date"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("workspaceId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("transactionType"),
				Token::UnitVariant {
					name: "TransactionType",
					variant: "payment",
				},
				Token::Str("paymentStatus"),
				Token::UnitVariant {
					name: "PaymentStatus",
					variant: "success",
				},
				Token::Str("description"),
				Token::None,
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetTransactionHistoryResponse {
				transactions: vec![Transaction {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					month: 5,
					amount: 6969,
					payment_intent_id: Some(
						"pm_6K95KhSGEPBh7GrIsWVB4pyV".to_string(),
					),
					date: Utc.timestamp_opt(1431648000, 0).unwrap().into(),
					workspace_id: Uuid::parse_str(
						"2aef18631ded45eb9170dc2166b30867",
					)
					.unwrap(),
					transaction_type: TransactionType::Payment,
					payment_status: PaymentStatus::Success,
					description: None,
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("transactions"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "Transaction",
					len: 9,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("month"),
				Token::I32(5),
				Token::Str("amount"),
				Token::U64(6969),
				Token::Str("paymentIntentId"),
				Token::Some,
				Token::Str("pm_6K95KhSGEPBh7GrIsWVB4pyV"),
				Token::Str("date"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("workspaceId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("transactionType"),
				Token::UnitVariant {
					name: "TransactionType",
					variant: "payment",
				},
				Token::Str("paymentStatus"),
				Token::UnitVariant {
					name: "PaymentStatus",
					variant: "success",
				},
				Token::Str("description"),
				Token::None,
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
