use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
	prelude::*,
	utils::{False, True},
};

/// The response from the OAuthIntrospect endpoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", untagged)]
pub enum OAuthIntrospectResponseType {
	/// If the access token is valid
	#[serde(rename_all = "snake_case")]
	Active {
		/// If the access token is valid
		active: True,
		/// The client ID of the third-party app
		client_id: String,
		/// The user ID of the user who owns the access token
		user_id: String,
		/// The scopes that the access token has
		scope: String,
		/// The expiry time of the access token
		#[serde(with = "time::serde::timestamp")]
		expires_at: OffsetDateTime,
	},
	/// If the access token is invalid
	#[serde(rename_all = "snake_case")]
	Inactive {
		/// If the access token is invalid
		active: False,
	},
}

macros::declare_api_endpoint!(
	/// The endpoint to get details about the access token and refresh token.
	///
	/// The third-party app can call this endpoint to get information about the
	/// access token and refresh token. This is useful for debugging and monitoring
	/// the tokens. It’s like asking, "Is this key still good?" This endpoint is
	/// used to get details about the access token and refresh token. It returns
	/// information about the token, such as the user ID, the scopes, and the expiry
	/// time.
	OAuthIntrospect,
	POST "/auth/oauth/introspect",
	request = {
		/// The access token that needs to be introspected
		#[serde(rename = "token")]
		pub token: String,
		/// The client ID of the third-party app
		#[serde(rename = "client_id")]
		pub client_id: String,
	},
	response = {
		/// The response from the OAuthIntrospect endpoint
		#[serde(flatten)]
		pub response: OAuthIntrospectResponseType,
	}
);
