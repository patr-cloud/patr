use crate::{prelude::*, utils::BearerToken};

macros::declare_api_endpoint!(
	// Access token 
	RenewAccessToken,
	GET "/auth/access-token",
	request = {
		#[serde(skip)]
		pub refresh_token: Uuid,
		pub login_id: Uuid,
	},
	response = {
		access_token: String,
	},
);
